use crate::{EvaluationResult, Scheduler};

use std::sync::Arc;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Task {}

impl Task {
    fn execute(self, _scheduler: Arc<Scheduler>) -> EvaluationResult {
        EvaluationResult::Store(1, 0.00)
    }
}

impl From<RawTask> for Task {
    fn from(_raw: RawTask) -> Self {
        // TODO
        Self {}
    }
}

impl From<Task> for RawTask {
    fn from(_task: Task) -> Self {
        // TODO
        vec![0x00u8].into()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RawTask {
    ptr: *mut u8,
    len: usize,
    cap: usize,
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
    pub fn execute(self, scheduler: Arc<Scheduler>) -> EvaluationResult {
        Task::from(self).execute(scheduler)
    }
}

unsafe impl Send for RawTask {}
unsafe impl Sync for RawTask {}
