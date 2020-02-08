use crate::{Evaluation, EvaluationVariant, Operation, OperationIterator, Scheduler};

use std::fmt;
use std::io;
use std::sync::Arc;
use std::task::Poll;

#[cfg(feature = "trace")]
use tracing::trace;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Task {
    ops: Vec<Operation>,
}

impl Task {
    pub fn from_reader<R: io::Read>(reader: R) -> Option<Self> {
        let mut iter = OperationIterator::new(reader);
        let mut ops = vec![];

        while let Some(task) = iter.next() {
            ops.push(task);

            match task {
                Operation::Return | Operation::Store(_) => break,
                _ => (),
            }
        }

        if ops.is_empty() {
            None
        } else {
            Some(Task { ops })
        }
    }

    /// # Panics
    ///
    /// Panics if the first operation is not multiple arguments.
    ///
    /// Panics is the last operation is not store or return
    fn execute(self, scheduler: Arc<Scheduler>, thread_id: usize) -> Evaluation {
        #[cfg(feature = "trace")]
        trace!("Executing task {:?}", self);

        match self.ops[0] {
            Operation::Return => {
                return Evaluation::new(EvaluationVariant::Return(0.00f64), thread_id)
            }
            Operation::Undefined => panic!("Undefined operation!"),
            _ => (),
        }

        let (lane, size) = self.ops[0].fetch_size();
        if size == 0 {
            panic!("The first operation of the task must be of multiple arguments");
        }

        let mut result = match scheduler.fetch_from_lane(lane, size as usize) {
            Poll::Ready(data) => self.ops[0].execute_multiple(data.as_slice()),
            Poll::Pending => {
                return Evaluation::new(EvaluationVariant::NotReady(self.into()), thread_id);
            }
        };

        for i in 1..self.ops.len() - 1 {
            let (lane, size) = self.ops[i].fetch_size();

            result = if size == 0 {
                self.ops[i].execute(result)
            } else {
                match scheduler.fetch_from_lane(lane, size as usize) {
                    Poll::Ready(data) => self.ops[i].execute_multiple(data.as_slice()),
                    Poll::Pending => {
                        return Evaluation::new(
                            EvaluationVariant::NotReady(self.into()),
                            thread_id,
                        );
                    }
                }
            };
        }

        match self.ops[self.ops.len() - 1] {
            Operation::Return => Evaluation::new(EvaluationVariant::Return(result), thread_id),
            Operation::Store(lane) => {
                Evaluation::new(EvaluationVariant::Store(lane, result), thread_id)
            }
            _ => panic!("The last operation of the task must be either return or store!"),
        }
    }
}

impl From<RawTask> for Task {
    fn from(raw: RawTask) -> Self {
        let bytes: Vec<u8> = raw.into();
        Task::from_reader(bytes.as_slice()).unwrap_or_default()
    }
}

impl From<Task> for RawTask {
    fn from(task: Task) -> Self {
        task.ops
            .into_iter()
            .fold(vec![], |mut v, op| {
                let b: Vec<u8> = op.into();
                v.extend(b);
                v
            })
            .into()
    }
}

#[derive(Debug, Copy, PartialEq)]
pub struct RawTask {
    ptr: *mut u8,
    len: usize,
    cap: usize,
}

impl RawTask {
    pub fn from_reader<R: io::Read>(reader: R) -> Option<Self> {
        Task::from_reader(reader).map(|t| t.into())
    }
}

impl Clone for RawTask {
    fn clone(&self) -> Self {
        let v = unsafe { Vec::from_raw_parts(self.ptr, self.len, self.cap) };

        let t = v.clone();
        std::mem::forget(v);

        t.into()
    }
}

impl fmt::Display for RawTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let task: Task = self.clone().into();
        write!(f, "{:?}", task)
    }
}

impl From<Vec<u8>> for RawTask {
    fn from(v: Vec<u8>) -> Self {
        let (ptr, len, cap) = v.into_raw_parts();
        Self { ptr, len, cap }
    }
}

impl Into<Vec<u8>> for RawTask {
    fn into(self) -> Vec<u8> {
        unsafe { Vec::from_raw_parts(self.ptr, self.len, self.cap) }
    }
}

impl RawTask {
    // Equivalent to Future::poll
    pub fn execute(self, scheduler: Arc<Scheduler>, thread_id: usize) -> Evaluation {
        let task: Task = self.clone().into();
        Task::from(self)
            .execute(scheduler, thread_id)
            .finish(task.into())
    }
}

unsafe impl Send for RawTask {}
unsafe impl Sync for RawTask {}
