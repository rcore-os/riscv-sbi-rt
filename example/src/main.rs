#![no_std]
#![no_main]

#[macro_use]
extern crate log;
#[macro_use]
extern crate opensbi_rt;

use log::LevelFilter;

#[no_mangle]
extern "C" fn main(hartid: usize, dtb: usize) {
    log::set_max_level(LevelFilter::Info);
    println!("Hello, OpenSBI!");
    info!("hartid={}, dtb={:#x}", hartid, dtb);
}
