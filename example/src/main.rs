#![no_std]
#![no_main]

#[macro_use]
extern crate log;
#[macro_use]
extern crate opensbi_rt;

#[no_mangle]
extern "C" fn main(hartid: usize, dtb: usize) {
    println!("Hello, OpenSBI! hartid={}, dtb={:#x}", hartid, dtb);
}
