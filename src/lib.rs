//! A mininal runtime / startup for OpenSBI on RISC-V.

#![no_std]
#![feature(alloc_error_handler)]
#![deny(warnings, missing_docs)]

extern crate alloc;

#[macro_use]
pub mod io;
mod log;
mod runtime;
pub mod sbi;
