# RusPiRo Channel

![CI](https://github.com/RusPiRo/ruspiro-channel/workflows/CI/badge.svg?branch=development)
[![Latest Version](https://img.shields.io/crates/v/ruspiro-channel.svg)](https://crates.io/crates/ruspiro-channel)
[![Documentation](https://docs.rs/ruspiro-channel/badge.svg)](https://docs.rs/ruspiro-channel)
[![License](https://img.shields.io/crates/l/ruspiro-channel.svg)](https://github.com/RusPiRo/ruspiro-channel#license)

## Usage

To use this crate simply add the dependency to your ``Cargo.toml`` file:

```toml
[dependencies]
ruspiro-channel = "||VERSION||"
```

The crate actually implements a *multi producer - multi consumer* (mpmc) channel only. The channel itself is non blocking but uses atomic operations. When used in a bare metal Raspberry Pi project it has to ensured that atomic operations can be used (configured and enabled MMU - see [`ruspiro-mmu`](https://crates.io/crates/ruspiro-mmu) crate).

The creation of a channel provides the sender and the receiver part of it. Both can be cloned if required.

```rust
use ruspiro_channel::mpmc;

fn main() {
  let (tx, rx) = mpmc::channel();

  tx.send(50u32);
  // this is non-blocking and returns `Err` if the channel has no more data
  if let Ok(value) = tx.recv() {
    assert_eq!(value, 50);
  }
}
```

With the `async` feature activated an async-channel can be created and the receiver is able to `await` available messages.

```rust
// it's assumed the crate is compiled with features = ["async"]
use ruspiro_channel::mpmc;

async fn foo() {
  let (tx, mut rx) = mpmc::async_channel();

  tx.send(42u32);
  while let Some(value) = rx.next().await {
    assert_eq!(value, 42);
  }
}
```

## Features

Feature   | Description
----------|-------------
**async** | Enables the `async` version of the channel implementation.
