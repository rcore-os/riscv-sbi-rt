//! A mininal runtime / startup for SBI (Supervisor Binary Interface) on RISC-V.

#![no_std]
#![feature(llvm_asm, global_asm)]
#![feature(alloc_error_handler)]
#![deny(warnings, missing_docs)]

extern crate alloc;

pub use riscv_sbi_rt_macros::{entry, interrupt};

use core::alloc::Layout;
use core::panic::PanicInfo;
use core::sync::atomic::*;
use linked_list_allocator::LockedHeap;
use riscv::register::{scause::Scause, sstatus::Sstatus, stvec};
use riscv_sbi::println;

/// Rust entry point (_start_rust)
///
/// # Safety
///
/// This function should only be called by startup assembly code
#[export_name = "_start_rust"]
pub unsafe extern "C" fn start_rust(hartid: usize, dtb: usize) -> ! {
    #[rustfmt::skip]
    extern "Rust" {
        // interrupt entry provided by assemble
        fn _start_trap_sbi();

        // Provided by supervisor implementation
        fn main(hartid: usize, dtb: usize);
    }

    static READY: AtomicBool = AtomicBool::new(false);
    if hartid == 0 {
        // todo: should be put into #[pre_init]
        riscv_sbi::log::init();
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP.as_ptr() as usize, HEAP_SIZE);
        READY.store(true, Ordering::Release);
    } else {
        while !READY.load(Ordering::Acquire) {
            spin_loop_hint();
        }
    }

    // Initialize trap hanlder
    // 使用 Direct 模式，将中断入口设置为 `_start_trap_sbi`
    stvec::write(_start_trap_sbi as usize, stvec::TrapMode::Direct);

    // Launch main function
    main(hartid, dtb);

    // Shotdown
    riscv_sbi::legacy::shutdown()
}

global_asm!(
    r#"
    .section .text.entry
    .globl _start
_start:
    mv tp, a0

    la sp, bootstack
    sll t0, a0, 14
    add sp, sp, t0

    call _start_rust

    .section .bss.stack
    .align 12
    .global bootstack
bootstack:
    .space 4096 * 4 * 4
    .global bootstacktop
bootstacktop:
"#
);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    halt();
}

#[no_mangle]
extern "C" fn abort() -> ! {
    panic!("abort!");
}

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

const HEAP_SIZE: usize = 0x1_00000;

static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[alloc_error_handler]
fn oom(layout: Layout) -> ! {
    panic!("out of memory: {:#x?}", layout);
}

fn halt() -> ! {
    loop {
        unsafe {
            llvm_asm!("wfi");
        }
    }
}

// supervisor interrupt handler

// Ref: https://os20-rcore-tutorial.github.io/rCore-Tutorial-deploy/docs/lab-1/guide/part-6.html
// todo: should we save all registers here or part of them only?

#[cfg(target_pointer_width = "64")]
global_asm!(
    "
    .macro SAVE reg, offset
        sd  \\reg, \\offset*8(sp)
    .endm
    .macro LOAD reg, offset
        ld  \\reg, \\offset*8(sp)
    .endm
"
);
#[cfg(target_pointer_width = "32")]
global_asm!(
    "
    .macro SAVE reg, offset
        sw  \\reg, \\offset*4(sp)
    .endm
    .macro LOAD reg, offset
        lw  \\reg, \\offset*4(sp)
    .endm
"
);
global_asm!(
    "
    .section .text
    .globl _start_trap_sbi
    .align 2  # 对齐到4字节
# 进入中断
# 保存 Context 并且进入 rust 中的中断处理函数 interrupt::handler::handle_interrupt()
_start_trap_sbi:
    # 在栈上开辟 Context 所需的空间
    addi    sp, sp, -34*8 # todo: REGBYTES here
    # 保存通用寄存器，除了 x0（固定为 0）
    SAVE    x1, 1
    addi    x1, sp, 34*8
    # 将原来的 sp（sp 又名 x2）写入 2 位置
    SAVE    x1, 2
    SAVE    x3, 3
    SAVE    x4, 4
    SAVE    x5, 5
    SAVE    x6, 6
    SAVE    x7, 7
    SAVE    x8, 8
    SAVE    x9, 9
    SAVE    x10, 10
    SAVE    x11, 11
    SAVE    x12, 12
    SAVE    x13, 13
    SAVE    x14, 14
    SAVE    x15, 15
    SAVE    x16, 16
    SAVE    x17, 17
    SAVE    x18, 18
    SAVE    x19, 19
    SAVE    x20, 20
    SAVE    x21, 21
    SAVE    x22, 22
    SAVE    x23, 23
    SAVE    x24, 24
    SAVE    x25, 25
    SAVE    x26, 26
    SAVE    x27, 27
    SAVE    x28, 28
    SAVE    x29, 29
    SAVE    x30, 30
    SAVE    x31, 31

    # 取出 CSR 并保存
    csrr    s1, sstatus
    csrr    s2, sepc
    SAVE    s1, 32
    SAVE    s2, 33

    # Context, scause 和 stval 作为参数传入
    mv a0, sp
    csrr a1, scause
    csrr a2, stval
    jal _start_trap_rust

    .globl __restore
# 离开中断
# 从 Context 中恢复所有寄存器，并跳转至 Context 中 sepc 的位置
__restore:
    # 恢复 CSR
    LOAD    s1, 32
    LOAD    s2, 33
    # 不恢复 scause 和 stval
    csrw    sstatus, s1
    csrw    sepc, s2

    # 恢复通用寄存器
    LOAD    x1, 1
    LOAD    x3, 3
    LOAD    x4, 4
    LOAD    x5, 5
    LOAD    x6, 6
    LOAD    x7, 7
    LOAD    x8, 8
    LOAD    x9, 9
    LOAD    x10, 10
    LOAD    x11, 11
    LOAD    x12, 12
    LOAD    x13, 13
    LOAD    x14, 14
    LOAD    x15, 15
    LOAD    x16, 16
    LOAD    x17, 17
    LOAD    x18, 18
    LOAD    x19, 19
    LOAD    x20, 20
    LOAD    x21, 21
    LOAD    x22, 22
    LOAD    x23, 23
    LOAD    x24, 24
    LOAD    x25, 25
    LOAD    x26, 26
    LOAD    x27, 27
    LOAD    x28, 28
    LOAD    x29, 29
    LOAD    x30, 30
    LOAD    x31, 31

    # 恢复 sp（又名 x2）这里最后恢复是为了上面可以正常使用 LOAD 宏
    LOAD    x2, 2
    sret
"
);

/// Saved trap frame
pub struct TrapFrame {
    /// 32 common registers
    pub x: [usize; 32],
    /// Sstatus register
    pub sstatus: Sstatus,
    /// Sepc register
    pub sepc: usize,
}

#[doc(hidden)]
#[no_mangle]
#[allow(unused_variables, non_snake_case)]
pub fn DefaultExceptionHandler(trap_frame: &TrapFrame, scause: Scause, stval: usize) -> ! {
    panic!("Default exception handler!");
}

#[doc(hidden)]
#[no_mangle]
#[allow(unused_variables, non_snake_case)]
pub fn DefaultInterruptHandler() {
    panic!("Default interrupt handler!");
}

/// Trap entry point rust (_start_trap_rust)
///
/// `scause` register is read to determine the cause of the trap.
/// Bit XLEN-1 indicates if it's an interrupt or an exception.
/// The result is examined and ExceptionHandler or one of the core interrupt handlers is called.
///
/// # Safety
///
/// This function should only be called by trap initializer assembly code.
#[export_name = "_start_trap_rust"]
pub unsafe fn start_trap_rust(trap_frame: *mut TrapFrame, scause: Scause, stval: usize) {
    extern "Rust" {
        fn ExceptionHandler(trap_frame: &mut TrapFrame, scause: Scause, stval: usize);
    }

    if scause.is_exception() {
        ExceptionHandler(&mut *trap_frame, scause, stval)
    } else {
        let code = scause.code();
        if code < __INTERRUPTS.len() {
            let h = &__INTERRUPTS[code];
            // if reserved, it would call DefaultHandler
            (h.handler)();
        } else {
            DefaultHandler();
        }
    }
}

// Interrupts; doc hidden, for checking `#[interrupt]` name only
#[doc(hidden)]
pub mod trap {
    pub enum Interrupt {
        UserSoft,
        SupervisorSoft,
        UserTimer,
        SupervisorTimer,
        UserExternal,
        SupervisorExternal,
    }

    pub use self::Interrupt as interrupt;
}

#[doc(hidden)]
pub union Vector {
    handler: unsafe fn(),
    reserved: unsafe fn(),
    invalid: unsafe fn(),
}

#[doc(hidden)]
#[no_mangle]
pub static __INTERRUPTS: [Vector; 12] = [
    Vector { handler: UserSoft },
    Vector {
        handler: SupervisorSoft,
    },
    Vector {
        reserved: DefaultHandler,
    },
    Vector {
        invalid: DefaultHandler,
    },
    Vector { handler: UserTimer },
    Vector {
        handler: SupervisorTimer,
    },
    Vector {
        reserved: DefaultHandler,
    },
    Vector {
        invalid: DefaultHandler,
    },
    Vector {
        handler: UserExternal,
    },
    Vector {
        handler: SupervisorExternal,
    },
    Vector {
        reserved: DefaultHandler,
    },
    Vector {
        invalid: DefaultHandler,
    },
];

extern "Rust" {
    fn UserSoft();
    fn SupervisorSoft();
    fn UserTimer();
    fn SupervisorTimer();
    fn UserExternal();
    fn SupervisorExternal();

    fn DefaultHandler();
}
