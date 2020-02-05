use crate::{Fifo, RawTask};

use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::task::Poll;
use std::thread::{self, JoinHandle};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvaluationResult {
    NewRawTask(RawTask),
    NotReady(RawTask),
    Store(u32, f64),
}

pub struct Scheduler {
    pub fifo: Arc<Fifo>,
    pub children: Vec<JoinHandle<()>>,
    pub buffered_tasks: Vec<RawTask>,
    pub lanes: Mutex<HashMap<u32, Vec<f64>>>,
}

impl Scheduler {
    pub fn run<I: 'static + IntoIterator<Item = u8>>(threads: usize, program: I)
    where
        <I as IntoIterator>::IntoIter: Send,
    {
        let fifo = Arc::new(Fifo::default());
        let (tx, rx) = mpsc::channel();

        let mut scheduler = Arc::new(Self {
            fifo: Arc::clone(&fifo),
            children: vec![],
            buffered_tasks: vec![],
            lanes: Mutex::new(HashMap::new()),
        });

        let children: Vec<JoinHandle<()>> = (0..threads)
            .map(|i| {
                let mut parent = Arc::clone(&scheduler);
                let tx_result = tx.clone();

                thread::spawn(move || {
                    let _child_index = i;

                    loop {
                        match Scheduler::get_mut_unchecked(&mut parent).pop_task() {
                            Some(task) => {
                                // TODO - This is critical, the channel cannot fail. Implement a better
                                // error handling
                                tx_result
                                    .send(task.execute(Arc::clone(&parent)))
                                    .expect("MPSC channel corrupted");
                            }
                            None => thread::yield_now(),
                        }
                    }
                })
            })
            .collect();

        unsafe {
            let scheduler = Arc::get_mut_unchecked(&mut scheduler);
            scheduler.children = children;

            scheduler.event_loop(rx, tx, program);
        }
    }

    fn parser_loop<I: IntoIterator<Item = u8>>(program: I, tx: mpsc::Sender<EvaluationResult>) {
        // TODO - Implement
        program.into_iter().for_each(|_byte| {
            tx.send(EvaluationResult::NewRawTask(b"Some data".to_vec().into()))
                .expect("Failed to submit a new task to the queue")
        });
    }

    fn event_loop<I: 'static + IntoIterator<Item = u8>>(
        &mut self,
        rx: mpsc::Receiver<EvaluationResult>,
        tx: mpsc::Sender<EvaluationResult>,
        program: I,
    ) where
        <I as IntoIterator>::IntoIter: Send,
    {
        let program = program.into_iter();
        let tx_tasks = tx.clone();
        thread::spawn(move || {
            Scheduler::parser_loop(program, tx_tasks);
        });

        loop {
            for task_result in rx.iter() {
                match task_result {
                    EvaluationResult::NewRawTask(task) => self.push_task(task),
                    EvaluationResult::NotReady(task) => self.push_task(task),
                    EvaluationResult::Store(idx, value) => self.push_lane(&tx, idx, value),
                }
            }

            if !self.buffered_tasks.is_empty() && !self.fifo.is_full() {
                for _ in 0..self.children.len() {
                    if let Some(task) = self.buffered_tasks.pop() {
                        if let Some(task) = Fifo::get_mut_unchecked(&mut self.fifo).push(task) {
                            self.buffered_tasks.push(task);
                        }
                    } else {
                        break;
                    }
                }
            }
        }
    }

    fn push_task(&mut self, task: RawTask) {
        if let Some(task) = Fifo::get_mut_unchecked(&mut self.fifo).push(task) {
            // This is not exposed, so there is no need to protect this with memory ordering nor
            // mutexes
            self.buffered_tasks.push(task);
        }
    }

    pub fn pop_task(&mut self) -> Option<RawTask> {
        Fifo::get_mut_unchecked(&mut self.fifo).pop()
    }

    pub fn get_mut_unchecked(arc: &mut Arc<Scheduler>) -> &mut Self {
        unsafe { Arc::get_mut_unchecked(arc) }
    }

    fn push_lane(&self, tx: &mpsc::Sender<EvaluationResult>, idx: u32, value: f64) {
        match self.lanes.lock() {
            Ok(mut lanes) => {
                if !lanes.contains_key(&idx) {
                    lanes.insert(idx, vec![value]);
                } else {
                    lanes.get_mut(&idx).map(|v| v.push(value));
                }
            }

            // Resource not available, reschedule evaluation
            Err(_) => tx
                .send(EvaluationResult::Store(idx, value))
                .expect("Evaluation result channel is corrupted"),
        }
    }

    pub fn fetch_from_lane(&self, idx: u32, items: usize) -> Poll<Vec<f64>> {
        match self.lanes.lock() {
            Ok(lanes) => {
                if !lanes.contains_key(&idx) {
                    return Poll::Pending;
                }

                lanes
                    .get(&idx)
                    .map(|v| {
                        if v.len() >= items {
                            Poll::Ready((0..items).map(|i| v[i]).collect())
                        } else {
                            Poll::Pending
                        }
                    })
                    .unwrap_or(Poll::Pending)
            }

            // Resource not available, reschedule evaluation
            Err(_) => Poll::Pending,
        }
    }
}
