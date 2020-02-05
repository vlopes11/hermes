use crate::RawTask;

use std::sync::{Arc, Mutex};

pub struct Fifo {
    pub head: Mutex<u8>,
    pub tail: u8,
    pub tasks: [Option<RawTask>; 256],
}

impl Default for Fifo {
    fn default() -> Self {
        Fifo {
            head: Mutex::new(0),
            tail: 0,
            tasks: [None; 256],
        }
    }
}

impl Fifo {
    /// Attempt to push a task to the queue. Returns `Some(task)` if the queue was full, and None
    /// if it was pushed
    ///
    /// It is expected that the scheduler will try to wake any sleeping consumer after this call
    pub fn push(&mut self, task: RawTask) -> Option<RawTask> {
        if self.is_full() {
            return Some(task);
        }

        // No need to set memory ordering, the consumer will always use `Option::take`.
        // Worst case scenario, he goes to sleep at the same moment there is a task push.
        // Since after any task push the scheduler will try to wake sleeping consumers, this will
        // not degrade the overall performance
        self.tasks[self.tail as usize].replace(task);
        self.tail = self.tail.wrapping_add(1);

        None
    }

    pub fn pop(&mut self) -> Option<RawTask> {
        // TODO - Change to Ordering::AcqRel - Critical section
        let head = match self.head.try_lock().map(|ref mut h| {
            let head = **h;
            if self.tasks[head as usize].is_some() {
                **h = head.wrapping_add(1);
            }
            head
        }) {
            Ok(h) => h,
            Err(_) => return None,
        };

        self.tasks[head as usize].take()
    }

    pub fn is_full(&self) -> bool {
        // TODO - Maybe change to Ordering::Relaxed? False positives are not a problem because the
        // worst case scenario is the task will be rescheduled
        let head = match self.head.try_lock().map(|h| *h) {
            Ok(h) => h,
            Err(_) => return true,
        };

        self.tail == head && self.tasks[self.tail as usize].is_some()
    }

    pub fn is_empty(&self) -> bool {
        // TODO - Maybe change to Ordering::Relaxed? False positives are not a problem because the
        // worst case scenario is the task will be rescheduled
        let head = match self.head.try_lock().map(|h| *h) {
            Ok(h) => h,
            Err(_) => return false,
        };

        self.tasks[head as usize].is_none()
    }

    pub fn get_mut_unchecked(arc: &mut Arc<Fifo>) -> &mut Self {
        unsafe { Arc::get_mut_unchecked(arc) }
    }
}

#[cfg(test)]
mod tests {
    use loom::thread;
    use std::sync::Arc;

    use crate::Fifo;

    #[test]
    fn head_and_tail() {
        let mut fifo = Fifo::default();

        assert!(fifo.pop().is_none());

        let head = fifo.head.lock().map(|ref h| **h).unwrap();
        assert_eq!(0, head);
        assert_eq!(0, fifo.tail);

        (0..fifo.tasks.len() - 1).for_each(|_| {
            fifo.push(b"Some task".to_vec().into());
        });

        let head = fifo.head.lock().map(|ref h| **h).unwrap();
        assert_eq!(0, head);
        assert_eq!(fifo.tasks.len() - 1, fifo.tail as usize);

        fifo.push(b"Some task".to_vec().into());
        let head = fifo.head.lock().map(|ref h| **h).unwrap();
        assert_eq!(0, head);
        assert_eq!(0, fifo.tail);

        (0..10).for_each(|_| {
            fifo.pop();
        });
        let head = fifo.head.lock().map(|ref h| **h).unwrap();
        assert_eq!(10, head);
        assert_eq!(0, fifo.tail);

        (0..5).for_each(|_| {
            fifo.push(b"Some task".to_vec().into());
        });
        let head = fifo.head.lock().map(|ref h| **h).unwrap();
        assert_eq!(10, head);
        assert_eq!(5, fifo.tail);
    }

    #[test]
    fn queue_push_pop() {
        // TODO - Loom is optimized for Atomic operations, which are not currently implemented
        loom::model(|| {
            let fifo_base = Arc::new(Fifo::default());

            let input = (0..256)
                .map(|i| format!("RawTask {:06}", i))
                .collect::<Vec<String>>();

            let mut fifo = Arc::clone(&fifo_base);
            {
                let fifo = Fifo::get_mut_unchecked(&mut fifo);

                for s in input.clone() {
                    fifo.push(s.into_bytes().into());
                }

                assert!(fifo
                    .push(format!("RawTask overflow").into_bytes().into())
                    .is_some());
            }

            let mut result = vec![];

            let mut children = vec![];

            for _ in 0..2 {
                let mut fifo = Arc::clone(&fifo_base);

                children.push(thread::spawn(move || {
                    let fifo = Fifo::get_mut_unchecked(&mut fifo);
                    let mut completed = vec![];

                    while let Some(task) = fifo.pop() {
                        completed.push(String::from_utf8(task.into()).unwrap());
                    }

                    completed
                }));
            }

            result.extend_from_slice(
                children
                    .into_iter()
                    .fold(vec![], |mut v, j| {
                        v.extend_from_slice(j.join().unwrap().as_slice());
                        v
                    })
                    .as_slice(),
            );

            result.sort();
            assert_eq!(input, result);
        });
    }
}
