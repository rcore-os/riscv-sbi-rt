//! A mininal runtime / startup for OpenSBI on RISC-V.

#![no_std]
#![feature(llvm_asm, global_asm)]
#![feature(alloc_error_handler)]
#![deny(warnings, missing_docs)]

extern crate alloc;

pub mod sbi;

#[macro_use]
#[doc(hidden)]
pub mod io;

mod log;
mod runtime;
pub mod trap;

pub use opensbi_rt_macros::{entry, interrupt};
