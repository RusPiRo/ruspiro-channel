/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: MIT OR Apache License 2.0
 **************************************************************************************************/

//! # Multi Producer Multi Consumer Channel
//!

#[cfg(feature = "async")]
mod r#async;
mod queue;

use alloc::sync::Arc;
use queue::*;
#[cfg(feature = "async")]
pub use r#async::*;

/// Create both sides of a mpmc channel
pub fn channel<T: 'static>() -> (Sender<T>, Receiver<T>) {
  let queue = Arc::new(Queue::new());
  (Sender::new(queue.clone()), Receiver::new(queue))
}

/// Sender that is using a queue to push messages/data that a receiver could work on
/// The sending part could be used by several cores in parallel to push stuff to the queue
#[repr(C)]
pub struct Sender<T> {
  inner: Arc<Queue<T>>,
}

#[doc(hidden)]
unsafe impl<T> Send for Sender<T> {}

impl<T: 'static> Sender<T> {
  pub fn new(inner: Arc<Queue<T>>) -> Self {
    Sender { inner }
  }

  pub fn send(&self, data: T) {
    self.inner.push(data)
  }
}

/// Enable cloning the Sender so it can be used at different cores, filling up the same queue
impl<T: 'static> Clone for Sender<T> {
  fn clone(&self) -> Sender<T> {
    Sender::new(self.inner.clone())
  }
}

/// Receiver that is using a queue to pop messages/data that has been pushed their for
/// processing
#[repr(C)]
pub struct Receiver<T> {
  inner: Arc<Queue<T>>,
}

impl<T: 'static> Receiver<T> {
  pub fn new(inner: Arc<Queue<T>>) -> Self {
    Receiver { inner }
  }

  pub fn recv(&self) -> Result<T, ()> {
    match self.inner.pop() {
      Pop::Data(v) => Ok(v),
      Pop::Empty | Pop::Intermediate => Err(()),
    }
  }
}

/// Enable cloning the receiver so it can be used at different cores to receive data from the same
/// queue
impl<T: 'static> Clone for Receiver<T> {
  fn clone(&self) -> Receiver<T> {
    Receiver::new(self.inner.clone())
  }
}
