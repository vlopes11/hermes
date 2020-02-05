#![feature(vec_into_raw_parts)]
#![feature(get_mut_unchecked)]

pub use crate::fifo::Fifo;
pub use crate::scheduler::{EvaluationResult, Scheduler};
pub use crate::task::RawTask;

pub mod fifo;
pub mod scheduler;
pub mod task;
