/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: MIT OR Apache License 2.0
 **************************************************************************************************/

//! # Multi Producer Multi Consumer
//!

use alloc::sync::Arc;
mod queue;
use queue::*;

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let queue = Arc::new(Queue::new());
    (Sender::new(queue.clone()), Receiver::new(queue))
}

/// Define a sender that is using a queue to push messages/data that a receiver could work on
/// The sending part could be used by several cores in parallel to push stuff to the queue
pub struct Sender<T> {
    inner: Arc<Queue<T>>,
}

#[doc(hidden)]
unsafe impl<T> Send for Sender<T> {}

impl<T> Sender<T> {
    pub fn new(inner: Arc<Queue<T>>) -> Self {
        Sender { inner }
    }

    pub fn send(&self, data: T) {
        self.inner.push(data)
    }
}

/// Enable cloning the Sender so it can be used at different cores, filling up the same queue
impl<T> Clone for Sender<T> {
    fn clone(&self) -> Sender<T> {
        Sender::new(self.inner.clone())
    }
}

/// Define a receiver that is using a queue to pop messages/data that has been pushed their for
/// processing
pub struct Receiver<T> {
    inner: Arc<Queue<T>>,
}

impl<T> Receiver<T> {
    pub fn new(inner: Arc<Queue<T>>) -> Self {
        Receiver { inner }
    }

    pub fn recv(&self) -> Result<T, ()> {
        //info!("get next mpc entry");
        match self.inner.pop() {
            Pop::Data(v) => Ok(v),
            Pop::Empty | Pop::Intermediate => Err(()),
        }
    }
}

/// Enable cloning the receiver so it can be used at different cores to receive data from the same
/// queue
impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Receiver<T> {
        Receiver::new(self.inner.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_use_channel() {
        let (tx, rx) = channel();
        tx.send(1u32);
        assert_eq!(rx.recv().unwrap(), 1u32);
    }
}
