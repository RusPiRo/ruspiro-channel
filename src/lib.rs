/***********************************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: MIT OR Apache License 2.0
 **********************************************************************************************************************/
//! # Bare metal (``no_std``) channel implementations
//!
//! The follwoing channel implementations require only an allocator to be present as it uses heap allocations.

#![doc(html_root_url = "https://docs.rs/ruspiro-channel/||VERSION||")]
#![no_std]

extern crate alloc;

pub mod mpmc;
