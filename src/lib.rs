#![no_std]
#![feature(asm, global_asm)]
#![feature(alloc_error_handler)]
#![deny(warnings)]

extern crate alloc;

#[macro_use]
pub mod io;
mod runtime;
pub mod sbi;
