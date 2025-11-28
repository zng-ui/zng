#![cfg(not(target_arch = "wasm32"))]

//! Async process API and worker.
//!
//! This module reexports the [`async-process`](https://docs.rs/async-process) for convenience.

#[cfg(ipc)]
pub mod worker;

pub use async_process::*;
