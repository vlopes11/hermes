#![feature(vec_into_raw_parts)]
#![feature(get_mut_unchecked)]

pub use crate::fifo::Fifo;
pub use crate::task::Task;

pub mod fifo;
pub mod task;
