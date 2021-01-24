/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: MIT OR Apache License 2.0
 **************************************************************************************************/
//! # Async Multi-Producer-Multi-Consumer Channel
//!
//!

use super::{channel, Receiver, Sender};
use alloc::sync::Arc;
use core::{
  pin::Pin,
  task::{Context, Poll},
};
use futures_util::{stream::Stream, task::AtomicWaker};

/// Create both sides of an asynchronous mpmc channel
pub fn async_channel<T: 'static>() -> (AsyncSender<T>, AsyncReceiver<T>) {
  let (tx, rx) = channel();
  let waker = Arc::new(AtomicWaker::new());
  (
    AsyncSender {
      tx,
      waker: waker.clone(),
    },
    AsyncReceiver { rx, waker },
  )
}

/// The sending part of a mpmc channel. This is used to send data through the channel to an receiver
pub struct AsyncSender<T: 'static> {
  tx: Sender<T>,
  waker: Arc<AtomicWaker>,
}

impl<T: 'static> AsyncSender<T> {
  /// Send data through the channel
  pub fn send(&self, data: T) {
    self.tx.send(data);
    self.waker.wake();
  }
}

/// Enable cloning the Sender so it can be used at different cores, filling up the same queue
impl<T: 'static> Clone for AsyncSender<T> {
  fn clone(&self) -> AsyncSender<T> {
    AsyncSender {
      tx: self.tx.clone(),
      waker: self.waker.clone(),
    }
  }
}

/// The receiving part of the channel. This can be used to retreive data that has been send from the sending part of it
pub struct AsyncReceiver<T: 'static> {
  rx: Receiver<T>,
  waker: Arc<AtomicWaker>,
}

/// Enable cloning the Sender so it can be used at different cores, filling up the same queue
impl<T: 'static> Clone for AsyncReceiver<T> {
  fn clone(&self) -> AsyncReceiver<T> {
    AsyncReceiver {
      rx: self.rx.clone(),
      waker: self.waker.clone(),
    }
  }
}

/// The asyncrounous channel receiving part acts like a stream. When waiting for new data to arrive call `next().await`
/// on the receiver.
impl<T: 'static> Stream for AsyncReceiver<T> {
  type Item = T;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    if let Ok(v) = self.rx.recv() {
      Poll::Ready(Some(v))
    } else {
      // how get woken? This should be done by the sender
      self.waker.register(cx.waker());
      Poll::Pending
    }
  }
}
