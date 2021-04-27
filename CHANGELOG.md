# Changelog

## :banana: v0.1.1

This is a maintenance release fixing an issue with the documentation generation on `doc.rs` and also adresses some depricated function calls on atomics.

- ### :wrench: Maintenance

  - Fix doc.rs build issue with the custom target in place
  - Change depricated calls on atomics to the correct new functions
  - provide a proper `Drop` implementation for the `Queue` do pop all remaining elements and thus free the aquired memory.

## :apple: v0.1.0

Initial version of the bare metal channel implementation. The baseline version provides implementation for a lock-free *Multi-Producer-Multi-Consumer* (MPMC) channel. With the `async` feature the crate provides also an async version of this channel allowing the receiver to await new data.

- ### :bulb: Features
  
  - initial lock-free MPMC channel implementation
  - async channel version

- ### :book: Documentation
  
  Provide the initial documentation of the crate
  