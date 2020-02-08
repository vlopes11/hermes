use crate::{Fifo, RawTask, Task};

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::sync::{
    mpsc::{self, Receiver, SyncSender as Sender, TryRecvError},
    Arc, Mutex,
};
use std::task::Poll;
use std::thread::{self, JoinHandle};
use std::time::Instant;

#[cfg(feature = "trace")]
use tracing::{error, trace};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Evaluation {
    pub variant: EvaluationVariant,
    pub thread: usize,
    pub start: Instant,
    pub end: Instant,
    pub task: Option<RawTask>,
}

impl Evaluation {
    pub fn new(variant: EvaluationVariant, thread: usize) -> Self {
        Self {
            variant,
            thread,
            start: Instant::now(),
            end: Instant::now(),
            task: None,
        }
    }

    pub fn finish(mut self, task: RawTask) -> Self {
        self.end = Instant::now();
        self.task.replace(task);
        self
    }

    pub fn dispatch(self, scheduler: &mut Scheduler, tx: &Sender<Evaluation>) -> Option<f64> {
        #[cfg(feature = "trace")]
        trace!("Received task {}", self);

        match self.variant {
            EvaluationVariant::NewRawTask(task) => scheduler.push_task(task),
            EvaluationVariant::NotReady(task) => scheduler.push_task(task),
            EvaluationVariant::Store(idx, value) => {
                scheduler.blocking_push_step(
                    idx,
                    self.task.map(|t| t.into()).unwrap_or_default(),
                    self.start,
                    self.end,
                );
                scheduler.push_lane(tx, idx, value, self)
            }
            EvaluationVariant::Sleep(idx) => scheduler.sleeping_worker.push(idx),
            EvaluationVariant::Return(result) => return Some(result),
        }

        #[cfg(feature = "trace")]
        trace!("Task submitted");

        None
    }
}

impl fmt::Display for Evaluation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Evaluation {{ variant: {}, thread: {} }}",
            self.variant, self.thread
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvaluationVariant {
    NewRawTask(RawTask),
    NotReady(RawTask),
    Store(u32, f64),
    Return(f64),
    Sleep(usize),
}

impl fmt::Display for EvaluationVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvaluationVariant::NewRawTask(task) => write!(f, "NewRawTask({})", task),
            EvaluationVariant::NotReady(task) => write!(f, "NotReady({})", task),
            _ => write!(f, "{:?}", self),
        }
    }
}

pub struct Scheduler {
    pub fifo: Arc<Fifo<RawTask>>,
    pub worker: Vec<JoinHandle<()>>,
    pub sleeping_worker: Vec<usize>,
    pub buffered_tasks: Vec<RawTask>,
    pub lanes: Mutex<HashMap<u32, Vec<f64>>>,
    pub history: Mutex<HashMap<u32, (Task, Instant, Instant)>>,
}

impl Scheduler {
    pub fn lane_initializer(lanes: Vec<(u32, Vec<f64>)>) -> Vec<u8> {
        let mut bytes = lanes.into_iter().fold(vec![], |mut bytes, (idx, items)| {
            let size = items.len() as u32;
            bytes.extend_from_slice(&size.to_le_bytes());
            bytes.extend_from_slice(&idx.to_le_bytes());

            items
                .into_iter()
                .for_each(|value| bytes.extend_from_slice(&value.to_le_bytes()));

            bytes
        });

        bytes.extend_from_slice(&[0x00u8; 4]);
        bytes
    }

    pub fn initialize_lanes_from_reader<R: io::Read>(mut reader: R) -> HashMap<u32, Vec<f64>> {
        let mut hm = HashMap::new();

        loop {
            let mut size = [0x00u8; 4];
            reader
                .read_exact(&mut size)
                .expect("Could not fetch the quantity of inputs for the lane initialization phase");

            let size = u32::from_le_bytes(size);
            if size == 0 {
                break;
            }

            let mut lane = [0x00u8; 4];
            reader
                .read_exact(&mut lane)
                .expect("Could not fetch the lane id for the lane initialization phase");

            let lane = u32::from_le_bytes(lane);

            let items: Vec<f64> = (0..size)
                .map(|_| {
                    let mut input = [0x00u8; 8];
                    reader
                        .read_exact(&mut input)
                        .expect("Could not read the items for the lane initialization phase");

                    f64::from_le_bytes(input)
                })
                .collect();

            hm.insert(lane, items);
        }

        hm
    }

    pub fn run<R: io::Read>(threads: usize, mut program: R) -> f64 {
        let fifo = Arc::new(Fifo::default());
        let (tx, rx) = mpsc::sync_channel(2048);

        let mut scheduler = Arc::new(Self {
            fifo: Arc::clone(&fifo),
            worker: vec![],
            sleeping_worker: vec![],
            buffered_tasks: vec![],
            lanes: Mutex::new(Scheduler::initialize_lanes_from_reader(&mut program)),
            history: Mutex::new(HashMap::new()),
        });

        let worker: Vec<JoinHandle<()>> = (0..threads)
            .map(|i| {
                let parent = Arc::clone(&scheduler);
                let tx_result = tx.clone();

                thread::spawn(move || thread_loop(i, parent, tx_result))
            })
            .collect();

        let result = unsafe {
            let tx_event = tx.clone();
            let result = thread::spawn(move || {
                let mut scheduler = Arc::get_mut_unchecked(&mut scheduler);
                scheduler.worker = worker;
                Scheduler::event_loop(scheduler, rx, tx_event)
            });

            Scheduler::parser_loop(program, tx);

            result
                .join()
                .expect("Failed to run the main scheduler loop")
        };

        result
    }

    fn parser_loop<R: io::Read>(mut program: R, tx: Sender<Evaluation>) {
        while let Some(task) = RawTask::from_reader(&mut program) {
            #[cfg(feature = "trace")]
            trace!("Parsed task {}", task);

            channel_blocking_send(&tx, Evaluation::new(EvaluationVariant::NewRawTask(task), 0));
        }
    }

    fn event_loop(
        scheduler: &mut Scheduler,
        rx: Receiver<Evaluation>,
        tx: Sender<Evaluation>,
    ) -> f64 {
        #[cfg(feature = "trace")]
        trace!("Initiating event loop");

        loop {
            loop {
                match rx.try_recv() {
                    Ok(evaluation) => {
                        if let Some(result) = evaluation.dispatch(scheduler, &tx) {
                            return result;
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => panic!("Channel is disconnected"),
                }
            }

            // Max attempts
            for _ in 0..5 {
                if scheduler.buffered_tasks.is_empty() || scheduler.fifo.is_full() {
                    break;
                }

                if let Some(task) = scheduler.buffered_tasks.pop() {
                    scheduler.push_task(task);
                }
            }

            // Max attempts
            for _ in 0..5 {
                if scheduler.sleeping_worker.is_empty() || scheduler.fifo.is_empty() {
                    break;
                }

                if let Some(worker) = scheduler.sleeping_worker.pop() {
                    #[cfg(feature = "trace")]
                    trace!(
                        "Awaking worker {:?}",
                        scheduler.worker[worker].thread().id()
                    );
                    scheduler.worker[worker].thread().unpark();
                }
            }
        }
    }

    fn push_task(&mut self, task: RawTask) {
        #[cfg(feature = "trace")]
        trace!("Pushing task {}", task);
        if let Some(task) = Fifo::get_mut_unchecked(&mut self.fifo).push(task) {
            #[cfg(feature = "trace")]
            trace!("Buffering task {}", task);
            self.buffered_tasks.push(task);
        } else {
            #[cfg(feature = "trace")]
            trace!("Rescheduling task {}", task);
        }
    }

    pub fn pop_task(&mut self) -> Option<RawTask> {
        #[cfg(feature = "trace")]
        trace!("{:?} Popping task", thread::current().id());

        Fifo::get_mut_unchecked(&mut self.fifo).pop()
    }

    pub fn get_mut_unchecked(arc: &mut Arc<Scheduler>) -> &mut Self {
        unsafe { Arc::get_mut_unchecked(arc) }
    }

    fn push_lane(&self, tx: &Sender<Evaluation>, idx: u32, value: f64, task: Evaluation) {
        #[cfg(feature = "trace")]
        trace!("Attempting to push {} to lane {}", value, idx);

        match self.lanes.try_lock() {
            Ok(mut lanes) => {
                if !lanes.contains_key(&idx) {
                    lanes.insert(idx, vec![value]);
                } else {
                    lanes.get_mut(&idx).map(|v| v.push(value));
                }

                #[cfg(feature = "trace")]
                trace!("Value {} pushed to lane {}", value, idx);
            }

            // Resource not available, reschedule evaluation
            Err(_) => {
                channel_blocking_send(&tx, task.clone());
            }
        }
    }

    fn blocking_push_step(&self, lane: u32, task: Task, start: Instant, end: Instant) {
        // TODO - Critical point, should be reworked
        for _ in 0..1000 {
            #[cfg(feature = "trace")]
            trace!("Attempting to push to tasks history");

            match self.history.try_lock() {
                Ok(mut history) => {
                    history.insert(lane, (task.clone(), start, end));

                    #[cfg(feature = "trace")]
                    trace!("Task history inserted");

                    return ();
                }

                // Resource not available, reschedule evaluation
                Err(_e) => {
                    #[cfg(feature = "trace")]
                    error!("Task history push failed: {}", _e);
                    thread::yield_now();
                }
            }
        }

        eprintln!("Channel send failed with max attempts");
        std::process::exit(1);
    }

    pub fn fetch_from_lane(&self, idx: u32, items: usize) -> Poll<Vec<f64>> {
        match self.lanes.try_lock() {
            Ok(mut lanes) => {
                if !lanes.contains_key(&idx) {
                    return Poll::Pending;
                }

                let data = lanes
                    .get(&idx)
                    .map(|v| {
                        if v.len() >= items {
                            let mut r = Vec::with_capacity(items);
                            r.extend_from_slice(&v[0..items]);
                            Poll::Ready(r)
                        } else {
                            Poll::Pending
                        }
                    })
                    .unwrap_or(Poll::Pending);

                if let Poll::Ready(_) = data {
                    lanes.remove(&idx);
                }

                data
            }

            // Resource not available, reschedule evaluation
            Err(_) => {
                #[cfg(feature = "trace")]
                trace!(
                    "{:?} Fetch from lane is pending: ({}, {})",
                    thread::current().id(),
                    idx,
                    items
                );

                thread::yield_now();
                Poll::Pending
            }
        }
    }
}

fn thread_loop(worker_index: usize, mut parent: Arc<Scheduler>, tx: Sender<Evaluation>) {
    loop {
        let result = match Scheduler::get_mut_unchecked(&mut parent).pop_task() {
            Some(task) => {
                #[cfg(feature = "trace")]
                trace!("{:?} Task received: {}", thread::current().id(), task);

                let result = task.execute(Arc::clone(&parent), worker_index);

                #[cfg(feature = "trace")]
                trace!("{:?} Task executed: {}", thread::current().id(), result);

                result
            }
            None => {
                #[cfg(feature = "trace")]
                trace!("Yielding thread {:?}", thread::current().id());

                Evaluation::new(EvaluationVariant::Sleep(worker_index), worker_index)
            }
        };

        channel_blocking_send(&tx, result);

        if let EvaluationVariant::Sleep(_) = result.variant {
            thread::park();
        }
    }
}

fn channel_blocking_send<T: Copy>(tx: &Sender<T>, data: T) {
    // TODO - Critical point, should be reworked
    for _ in 0..1000 {
        match tx.send(data) {
            Ok(_) => return (),
            Err(_e) => {
                #[cfg(feature = "trace")]
                error!("Channel send failed: {}", _e);
                thread::yield_now();
            }
        }
    }

    eprintln!("Channel send failed with max attempts");
    std::process::exit(1);
}
