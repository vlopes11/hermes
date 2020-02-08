#![feature(vec_into_raw_parts)]
#![feature(get_mut_unchecked)]

pub use crate::fifo::{Fifo, FIFO_CAPACITY};
pub use crate::operation::{Operation, OperationIterator};
pub use crate::scheduler::{Evaluation, EvaluationVariant, Scheduler};
pub use crate::task::{RawTask, Task};

pub mod fifo;
pub mod generator;
pub mod operation;
pub mod scheduler;
pub mod task;
