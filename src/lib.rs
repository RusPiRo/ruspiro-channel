/***********************************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **********************************************************************************************************************/
#![doc(html_root_url = "https://docs.rs/ruspiro-channel/0.1.0")]
#![cfg_attr(not(any(test, doctest)), no_std)]

//! # Bare metal (``no_std``) channel implementations
//!
//! The follwoing channel implementations require only an allocator to be present as it uses heap allocations for the
//! ``Arc`` types.

extern crate alloc;

pub mod mpmc;