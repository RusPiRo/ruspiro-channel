/***********************************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **********************************************************************************************************************/
//! # Multi Producer Multi Consumer Queue
//!
//!

use alloc::boxed::Box;
use core::sync::atomic::{AtomicPtr, Ordering};

/// Representation of an entry in the [Queue]
#[derive(Debug)]
#[repr(align(16))]
struct Node<T> {
    /// Pointer to the next node in the [Queue]. This allows for storing the items as a single
    /// linked list ensuring Atomic access to the value from different cores
    next: AtomicPtr<Node<T>>,
    /// The actually stored value of this node that could be retreived while [pop]ing from the [Queue]
    /// or set while [push]ing to it
    value: Option<T>,
}

impl<T> Node<T> {
    /// create a new [Node] on the heap and return the mutable pointer to it
    unsafe fn new(value: Option<T>) -> *mut Node<T> {
        Box::into_raw(Box::new(Node {
            next: AtomicPtr::new(core::ptr::null_mut()),
            value,
        }))
    }
}

/// The actual Queue
pub struct Queue<T> {
    /// The head of the [Queue] refers to the first node that can be retrieved with [pop]
    head: AtomicPtr<Node<T>>,
    /// The tail of the [Queue] refers to the last node of the [Queue] new items can be [pushed]ed
    /// after
    tail: AtomicPtr<Node<T>>,
}

// The Queue is Send and Sync as it can be savely used accross cores
unsafe impl<T: Send> Send for Queue<T> {}
unsafe impl<T: Sync> Sync for Queue<T> {}

#[derive(Debug)]
pub enum Pop<T> {
    /// When [pop]ing an entry from an empty [Queue] this is the result
    Empty,
    /// The element that was [pop]ed from the [Queue]
    Data(T),
    /// Intermediate state of [pop]ing a new node, the requestor should retry the [pop] attemp
    Intermediate,
}

impl<T> Queue<T> {
    /// create a new empty [Queue]
    pub fn new() -> Self {
        // create the initial root node that marks the beginning of the [Queue]
        // this node does not store any value
        let root = unsafe { Node::new(None) };
        Queue {
            head: AtomicPtr::new(root),
            tail: AtomicPtr::new(root),
        }
    }

    /// Push a new element to the tail of the [Queue]
    /// The [Queue] consumes this element
    pub fn push(&self, value: T) {
        unsafe {
            // create a new node
            let node = Node::new(Some(value));
            // store this node at the tail giving the current last entry using atomic operation
            // to ensure any other core see's the new value of this store when trying to push a new
            // node in the meantime. So they already see the new tail
            let prev = self.tail.swap(node, Ordering::SeqCst);
            // finally we update the next pointer in the old tail to ensure proper linkage to our new
            // node from here this node is "pop-able"
            (*prev).next.store(node, Ordering::Relaxed);
        }
    }

    /// Pop an element from the head of the [Queue]
    pub fn pop(&self) -> Pop<T> {
        // for the time beeing popping from the queue is not "lockfree". However, as this happens
        // only outside of an interrupt this might be fine

        let result = unsafe {
            // get the first "pop-able" node from the head and replace it with a dummy node
            // to ensure simultaneus "pop's" does not happen in a strange order
            let dummy = Node::new(None);
            // if this operation finishes any other core that tries to pop a Node will get the
            // dummy Node
            let head = self.head.swap(dummy, Ordering::SeqCst);
            //info!("swapped");
            //info!("swapped head {:#x?} - h {:#x?}", self.head, head);
            //let head = self.head.load(Ordering::Acquire);
            // this first entry is either the queue start marker/dummy node or the entry that has
            // been popped last. So the node we will pop now is it's next one
            let next = (*head).next.load(Ordering::Relaxed);
            if !next.is_null() {
                // there is an item that could be retrieved from the queue
                // move the head of the queue to the entry we are about to retrieve
                // this will now let any other core see the new head, the dummy is gone
                self.head.store(next, Ordering::Release);
                //info!("stored");
                if (*next).value.is_none() {
                    // this should never happen!
                    //error!("next value is none !? at {:#x?}", next);
                    //warn!("origin head: {:#x?}", head,);
                    //warn!("actual head: {:#x?}", self.head);
                    //warn!("Dummy: {:#x?}", dummy);

                    // destruct the dummy node to release the memory
                    let _: Box<Node<T>> = Box::from_raw(dummy);

                    Pop::Intermediate
                } else {
                    let value = (*next).value.take().unwrap();
                    // we have stored a Box as rawpointer in the list
                    // construct the Box from the raw pointer and "forget" about it to properly
                    // destruct the same
                    let _: Box<Node<T>> = Box::from_raw(head);
                    // destruct the dummy node to release the memory
                    let _: Box<Node<T>> = Box::from_raw(dummy);
                    Pop::Data(value)
                }
            } else {
                // if we get here the current head does not have any next item, so the queue is actually
                // empty, or does contain the dummy entry, so restore the original head entry
                let dummy = self.head.swap(head, Ordering::SeqCst);
                //info!("swapped back");
                // destruct the dummy node to release the memory
                let _: Box<Node<T>> = Box::from_raw(dummy);
                // if the head and tail where the same the Queue was really empty, otherwise it was an
                // intermediate state
                if self.tail.load(Ordering::Relaxed) == head {
                    Pop::Empty
                } else {
                    Pop::Intermediate
                }
            }
        };

        result
    }
}

impl<T> Drop for Queue<T> {
    fn drop(&mut self) {
        // dropping the queue means we need to drop all contained items
        // as they have allocated memory
        unsafe {
            let mut current = self.head.load(Ordering::Relaxed);
            while !current.is_null() {
                let next = (*current).next.load(Ordering::Relaxed);
                let _: Box<Node<T>> = Box::from_raw(current);
                current = next;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;
    use assert_matches::*;
    use std::{thread, time};

    #[test]
    fn pop_empty_queue() {
        let queue = Queue::<()>::new();
        assert_matches!(queue.pop(), Pop::Empty);
    }

    #[test]
    fn simple_queue_push_and_pop() {
        let queue = Queue::new();
        queue.push(1u32);
        queue.push(2u32);
        assert_matches!(queue.pop(), Pop::Data(1u32));
        assert_matches!(queue.pop(), Pop::Data(2u32));
        assert_matches!(queue.pop(), Pop::Empty);
    }

    #[test]
    fn threaded_queue_push_and_sequential_pop() {
        let nthreads = 5;
        let nmsgs = 10;
        let queue = Queue::new();
        // wrap the Queue in an Arc to be shared
        let queue = Arc::new(queue);
        for t in 0..nthreads {
            let q = queue.clone();
            thread::spawn(move || {
                for i in 0..nmsgs {
                    q.push(i + (t * nmsgs));
                }
            });
        }

        thread::sleep(time::Duration::from_millis(nthreads * nmsgs * 100));

        let mut i = 0;
        while i < nthreads * nmsgs {
            match queue.pop() {
                Pop::Empty => std::println!("Empty at {}", i),
                Pop::Intermediate => std::println!("Intermediate at {}", i),
                Pop::Data(v) => {
                    std::println!("pop {}", v);
                    i += 1;
                }
            }
        }
    }

    #[test]
    fn sequential_queue_push_and_threaded_pop() {
        let nthreads = 5;
        let nmsgs = 10;
        let queue = Queue::new();
        // wrap the Queue in an Arc to be shared
        let queue = Arc::new(queue);
        for t in 0..nthreads {
            for i in 0..nmsgs {
                queue.push(i + (t * nmsgs));
            }
        }

        for _ in 0..nthreads {
            let q = queue.clone();
            thread::spawn(move || {
                for i in 0..nmsgs {
                    match q.pop() {
                        Pop::Empty => std::println!("Empty at {}", i),
                        Pop::Intermediate => std::println!("Intermediate at {}", i),
                        Pop::Data(v) => std::println!("pop {}", v),
                    };
                }
            });
        }

        thread::sleep(time::Duration::from_millis(nthreads * nmsgs * 100));
    }

    #[test]
    fn threaded_queue_push_and_threaded_pop() {
        let nthreads = 5;
        let nmsgs = 10;
        let queue = Queue::new();
        // wrap the Queue in an Arc to be shared
        let queue = Arc::new(queue);
        for t in 0..nthreads {
            let q = queue.clone();
            thread::spawn(move || {
                for i in 0..nmsgs {
                    q.push(i + (t * nmsgs));
                }
            });
        }

        thread::sleep(time::Duration::from_millis(nthreads * nmsgs * 100));

        let queue2 = Queue::new();
        let queue2 = Arc::new(queue2);
        for _ in 0..nthreads {
            let q = queue.clone();
            let q2 = queue2.clone();
            thread::spawn(move || {
                for i in 0..nmsgs {
                    match q.pop() {
                        Pop::Empty => std::println!("Empty at {}", i),
                        Pop::Intermediate => std::println!("Intermediate at {}", i),
                        Pop::Data(v) => q2.push(v),
                    };
                }
            });
        }

        thread::sleep(time::Duration::from_millis(nthreads * nmsgs * 100));

        let mut i = 0;
        while i < nthreads * nmsgs {
            match queue2.pop() {
                Pop::Empty => std::println!("Empty at {}", i),
                Pop::Intermediate => std::println!("Intermediate at {}", i),
                Pop::Data(v) => {
                    std::println!("pop {}", v);
                    i += 1;
                }
            }
        }
    }
}
