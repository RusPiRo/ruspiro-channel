/***********************************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: MIT OR Apache License 2.0
 **********************************************************************************************************************/
//! # Multi Producer Multi Consumer Queue
//!
//! This is an unbounded implementation of a MPMC queue. This is not using an array with a fixed size as a ringbuffer
//! that is typically used for bounded MPMC queues and thus limitting the number of elements the queue can handle.
//! This implementation uses a single linked list to store its elements.
//! Elements are always pushed to the back of the queue and poped from the front thus providing a FIFO buffer
//!
//! The implementation tries to be lockfree and only working with atomic operations instead of Mutex or other
//! data locks.
//!

use alloc::boxed::Box;
use core::{
  ptr,
  sync::atomic::{AtomicPtr, Ordering},
};
use ruspiro_arch_aarch64::instructions::*;

/// Representation of an entry in the [Queue]
#[derive(Debug)]
#[repr(align(16))]
struct Node<T: Sized> {
  /// Pointer to the next node in the [Queue].
  next: AtomicPtr<Node<T>>,
  /// The actually stored value of this node.
  value: Option<T>,
}

impl<T: Sized> Node<T> {
  /// create a new [Node] on the heap
  fn new(value: T) -> Box<Node<T>> {
    Box::new(Node {
      next: AtomicPtr::new(core::ptr::null_mut()),
      value: Some(value),
    })
  }
}

#[derive(Debug)]
pub enum Pop<T: Sized + 'static> {
  /// When popping an entry from an empty [Queue] this is the result
  Empty,
  /// The element that was popped from the [Queue]
  Data(T),
  /// Intermediate state of popping a new node, the requestor should retry the [pop] attempt
  Intermediate,
}

/// The actual Queue
pub struct Queue<T: Sized + 'static> {
  /// The head contains the pointer to the node that has been written last. Pushing to the queue will adjust
  /// the head.
  head: AtomicPtr<Node<T>>,
  /// The tail contains the pointer to the node that need to be read first. Popping from the queue will adjust
  /// tail
  tail: AtomicPtr<Node<T>>,
}

impl<T: Sized + 'static> Queue<T> {
  /// create a new empty [Queue]
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    // at the beginning of the lifetime of the queue the head and tail does not point anywhere
    Self {
      head: AtomicPtr::new(ptr::null_mut()),
      tail: AtomicPtr::new(ptr::null_mut()),
    }
  }

  /// Push a new element to the end of the [Queue]. The queue takes ownership of the value passed
  pub fn push(&self, value: T) {
    // 1. create a new node as raw pointer to ensure the node is not dropped when pushed to the queue
    //    The heap will be freed when the node is popped and converted back into a Box using Box::from_raw
    let node = Box::into_raw(Node::new(value));
    // 2. exchange the head with the new node
    dmb();
    let old_node = self.head.swap(node, Ordering::AcqRel);
    dsb();
    // 3. let the old node know it's next node.
    if !old_node.is_null() {
      unsafe {
        (*old_node).next.store(node, Ordering::SeqCst);
      }
    }
    // 4. if the tail is not yet pointing anywhere set the tail to the node just inserted
    dmb();
    // we can ignore the result of this operation. Err means the tail was alread set - so no need to update
    let _ = self
      .tail
      .compare_exchange(ptr::null_mut(), node, Ordering::AcqRel, Ordering::Relaxed);
    dsb();
  }

  /// Pop an element from the top of the [Queue]
  pub fn pop(&self) -> Pop<T> {
    // 1. swap the tail with an empty pointer to indicate nothing to read at the moment
    dmb();
    let node = self.tail.swap(ptr::null_mut(), Ordering::AcqRel);
    dsb(); // from this moment all cores/thread accessing tail will see it as "empty"
    if node.is_null() {
      return Pop::Intermediate;
    }
    // 2. if the node we popped last is the one sitting on head we have processed all nodes thus require to
    //    clean the head, otherwise dropping the node at the end of the pop would lead to access of freed memory
    //    when a new node is pushed
    // we can ignore the result of the operation as Err just indicates that we have not yet reached the HEAD and thus
    // do not want to do anything
    let _ = self
      .head
      .compare_exchange(node, ptr::null_mut(), Ordering::AcqRel, Ordering::Relaxed);
    dsb();

    // 3. re-construct the boxed node from the raw pointer
    let node = unsafe { Box::from_raw(node) };

    // 4. if this node has a follow-up node place this one into the tail
    let next_node = node.next.load(Ordering::Acquire);
    if !next_node.is_null() {
      // TODO: Check if we need to handle the case where tail is not null at this moment. We kind of expect it to be
      // null as we have swapped the current tail out and replaced with null at the beginning of the function. If the
      // value would now be any different from null this may indicate an implementation issue as another core would have
      // updated the tail (which should not happen) and thus the tail of the popped node will get lost.
      let _ = self.tail.compare_exchange(
        ptr::null_mut(),
        next_node,
        Ordering::AcqRel,
        Ordering::Relaxed,
      );
      dsb(); // from this moment all cores/thread accessing the tail will see a proper node to pop
    }

    // 5. get the value from the node and return it
    // if the node does not contain a value panicing is fine as this means the same node has been popped twice
    // which is an implementation error
    let value = node.value.unwrap();
    Pop::Data(value)
  }
}

impl<T: Sized + 'static> Drop for Queue<T> {
  fn drop(&mut self) {
    // dropping the queue means we need to drop all contained items
    // as they have allocated memory
    while let Pop::Data(_) = self.pop() {}
  }
}
