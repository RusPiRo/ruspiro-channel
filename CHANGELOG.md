# Changelog

## :apple: v0.1.0

Initial version of the bare metal channel implementation. The baseline version provides implementation for a lock-free *Multi-Producer-Multi-Consumer* (MPMC) channel. With the `async` feature the crate provides also an async version of this channel allowing the receiver to await new data.

- ### :bulb: Features
  
  - initial lock-free MPMC channel implementation
  - async channel version

- ### :book: Documentation
  
  Provide the initial documentation of the crate
  