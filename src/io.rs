use crate::sbi;
use core::fmt::{self, Write};
use spin::Mutex;

struct Stdout;

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            sbi::console_putchar(c as u8 as usize);
        }
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    static STDOUT: Mutex<Stdout> = Mutex::new(Stdout);
    STDOUT.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::io::_print(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
