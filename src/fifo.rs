#[cfg(feature = "trace")]
use tracing::trace;

#[cfg(test)]
use loom::sync::atomic::{AtomicU8, AtomicUsize};
#[cfg(not(test))]
use std::sync::atomic::{AtomicU8, AtomicUsize};

use std::fmt;
use std::sync::{atomic::Ordering, Arc, Mutex};
use std::thread;

pub const FIFO_CAPACITY: usize = 256;

pub struct Fifo<T: Copy + fmt::Display> {
    pub head: Mutex<u8>,
    pub tail: AtomicU8,
    pub len: AtomicUsize,
    pub pub_len: usize,
    pub data: [Option<T>; FIFO_CAPACITY],
}

impl<T: Copy + fmt::Display> Default for Fifo<T> {
    fn default() -> Self {
        Fifo {
            head: Mutex::new(0),
            tail: AtomicU8::new(0),
            len: AtomicUsize::new(0),
            pub_len: 0,
            data: [None; FIFO_CAPACITY],
        }
    }
}

impl<T: Copy + fmt::Display> Fifo<T> {
    /// Attempt to push data to the queue. Returns `Some(T)` if the queue was full, and None
    /// if it was pushed
    ///
    /// It is expected that the scheduler will try to wake any sleeping consumer after this call
    pub fn push(&mut self, data: T) -> Option<T> {
        #[cfg(feature = "trace")]
        trace!("Pushing data {}", data);

        if self.is_full() {
            #[cfg(feature = "trace")]
            trace!("Fifo is full, returning data {}", data);
            return Some(data);
        }

        let mut pub_len = self.len.fetch_add(1, Ordering::AcqRel);
        if pub_len <= FIFO_CAPACITY {
            pub_len += 1;
        }
        self.pub_len = pub_len;

        let tail = self.tail.fetch_add(1, Ordering::AcqRel);
        self.data[tail as usize].replace(data);

        None
    }

    #[cfg(test)]
    pub fn tail(&self) -> u8 {
        self.tail.load(Ordering::AcqRel)
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            #[cfg(feature = "trace")]
            trace!("Data pop attempt with empty set");

            return None;
        }

        #[cfg(feature = "trace")]
        trace!("Popping data");

        let mut inc_head = 0;
        while self
            .head
            .try_lock()
            .map(|ref mut h| {
                let head = **h;

                if self.data[head as usize].is_some() {
                    **h = head.wrapping_add(1);
                }

                inc_head = head;
            })
            .is_err()
        {
            #[cfg(feature = "trace")]
            trace!("Yielding head update");
            thread::yield_now();
        }

        let mut pub_len = self.len.fetch_sub(1, Ordering::AcqRel);
        if pub_len > 0 {
            pub_len -= 1;
        }

        self.pub_len = pub_len;
        self.data[inc_head as usize].take()
    }

    pub fn is_full(&self) -> bool {
        self.pub_len == FIFO_CAPACITY
    }

    pub fn is_empty(&self) -> bool {
        self.pub_len == 0
    }

    pub fn len(&self) -> usize {
        self.pub_len
    }

    pub fn head(&self) -> u8 {
        loop {
            match self.head.try_lock().map(|h| *h) {
                Ok(h) => return h,
                Err(_e) => {
                    #[cfg(feature = "trace")]
                    trace!("Yielding head fetch");
                    thread::yield_now()
                }
            }
        }
    }

    pub fn get_mut_unchecked(arc: &mut Arc<Fifo<T>>) -> &mut Self {
        unsafe { Arc::get_mut_unchecked(arc) }
    }
}

#[cfg(test)]
mod tests {
    use loom::thread;
    use std::sync::Arc;

    use crate::{Fifo, RawTask};

    #[test]
    fn head_and_tail() {
        loom::model(|| {
            let mut fifo: Fifo<RawTask> = Fifo::default();

            assert!(fifo.pop().is_none());

            let head = fifo.head.lock().map(|ref h| **h).unwrap();
            assert_eq!(0, head);
            assert_eq!(0, fifo.tail());

            (0..fifo.data.len() - 1).for_each(|_| {
                fifo.push(b"Some data".to_vec().into());
            });

            let head = fifo.head.lock().map(|ref h| **h).unwrap();
            assert_eq!(0, head);
            assert_eq!(fifo.data.len() - 1, fifo.tail() as usize);

            fifo.push(b"Some data".to_vec().into());
            let head = fifo.head.lock().map(|ref h| **h).unwrap();
            assert_eq!(0, head);
            assert_eq!(0, fifo.tail());

            (0..10).for_each(|_| {
                fifo.pop();
            });
            let head = fifo.head.lock().map(|ref h| **h).unwrap();
            assert_eq!(10, head);
            assert_eq!(0, fifo.tail());

            (0..5).for_each(|_| {
                fifo.push(b"Some data".to_vec().into());
            });
            let head = fifo.head.lock().map(|ref h| **h).unwrap();
            assert_eq!(10, head);
            assert_eq!(5, fifo.tail());
        });
    }

    #[test]
    fn queue_push_pop() {
        loom::model(|| {
            let fifo_base: Arc<Fifo<RawTask>> = Arc::new(Fifo::default());

            let input = (0..256)
                .map(|i| format!("Data {:06}", i))
                .collect::<Vec<String>>();

            let mut fifo = Arc::clone(&fifo_base);
            {
                let fifo = Fifo::get_mut_unchecked(&mut fifo);

                for s in input.clone() {
                    fifo.push(s.into_bytes().into());
                }

                assert!(fifo
                    .push(format!("Data overflow").into_bytes().into())
                    .is_some());
            }

            let mut result = vec![];

            let mut children = vec![];

            for _ in 0..2 {
                let mut fifo = Arc::clone(&fifo_base);

                children.push(thread::spawn(move || {
                    let fifo = Fifo::get_mut_unchecked(&mut fifo);
                    let mut completed = vec![];

                    while let Some(data) = fifo.pop() {
                        completed.push(String::from_utf8(data.into()).unwrap());
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
