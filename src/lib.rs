//! A mininal runtime / startup for Supervisor Binary Interface (SBI) implementations on RISC-V.

#![no_std]
#![feature(llvm_asm, global_asm)]
#![feature(alloc_error_handler)]
#![deny(warnings, missing_docs)]

extern crate alloc;

pub use riscv_sbi_rt_macros::{entry, interrupt, pre_init};

use core::alloc::Layout;
use core::panic::PanicInfo;
use core::sync::atomic::*;
use riscv::register::{scause::Scause, sstatus::Sstatus, stvec};
use riscv_sbi::println;

#[export_name = "error: riscv-sbi-rt appears more than once in the dependency graph"]
#[doc(hidden)]
pub static __ONCE__: () = ();

#[cfg(target_pointer_width = "32")]
extern "Rust" {
    // Boundaries of the .bss section
    static mut _ebss: u32;
    static mut _sbss: u32;

    // Boundaries of the .data section
    static mut _edata: u32;
    static mut _sdata: u32;

    // Initial values of the .data section (stored in Flash)
    static _sidata: u32;
}

#[cfg(target_pointer_width = "64")]
extern "Rust" {
    // Boundaries of the .bss section
    static mut _ebss: u64;
    static mut _sbss: u64;

    // Boundaries of the .data section
    static mut _edata: u64;
    static mut _sdata: u64;

    // Initial values of the .data section (stored in Flash)
    static _sidata: u64;
}

/// Rust entry point (_start_rust)
///
/// # Safety
///
/// This function should only be called by startup assembly code
#[export_name = "_start_rust"]
pub unsafe extern "C" fn start_rust(hartid: usize, dtb: usize) -> ! {
    #[rustfmt::skip]
    extern "C" {
        // interrupt entry provided by assemble
        fn _start_trap_sbi();

        // called once before bss and data is initialized
        fn __pre_init();

        // multi-processing hook function
        // must return true for only one hart which will initialize memory
        // and execute `pre_init` function
        // todo: finish design
        fn _mp_hook(hartid: usize, dtb: usize) -> bool;

        // entry function by supervisor implementation
        fn main(hartid: usize, dtb: usize);
    }

    static READY: AtomicBool = AtomicBool::new(false);
    if _mp_hook(hartid, dtb) {
        __pre_init();

        r0::zero_bss(&mut _sbss, &mut _ebss);
        r0::init_data(&mut _sdata, &mut _edata, &_sidata);

        riscv_sbi::log::init();

        READY.store(true, Ordering::Release);
    } else {
        while !READY.load(Ordering::Acquire) {
            spin_loop_hint();
        }
    }

    // Initialize trap hanlder
    // Use RISC-V defined Default mode, using trap entry `_start_trap_sbi`
    stvec::write(_start_trap_sbi as usize, stvec::TrapMode::Direct);

    // Launch main function
    main(hartid, dtb);

    // Shutdown
    riscv_sbi::legacy::shutdown()
}

global_asm!(
    r#"
    .section .text.entry
    .globl _start
_start:
    /* a0: hart id */
    /* a1: device tree root */
    
    /* Setup global pointer */
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop

    /* Check hard id limit */
    /* Do not read mhartid, here's supervisor level, would result in exception */
    lui t0, %hi(_max_hart_id)
    add t0, t0, %lo(_max_hart_id)
    bgtu a0, t0, _start_abort
    
    // mv tp, a0 /* todo: thread pointer */

    /* Prepare hart for each stack */
    /* Load symbols */
    la sp, _stack_start
    lui t0, %hi(_hart_stack_size)
    add t0, t0, %lo(_hart_stack_size)

    /* Calculate stack address */
    .ifdef __riscv_mul 
    mul t0, a0, t0
    .else
    beqz a0, 2f  /* jump if single-hart (a0 equals zero) */
    mv t1, a0
    mv t2, t0
1:
    add t0, t0, t2
    addi t1, t1, -1
    bnez t1, 1b
2:
    .endif

    /* Load stack address for this hart */
    sub sp, sp, t0

    # 计算 boot_page_table 的物理页号
    lui t0, %hi(boot_page_table)
    li t1, 0xffffffff00000000
    sub t0, t0, t1
    srli t0, t0, 12
    # 8 << 60 是 satp 中使用 Sv39 模式的记号
    li t1, (8 << 60)
    or t0, t0, t1
    # 写入 satp 并更新 TLB
    csrw satp, t0
    sfence.vma

    /* Convert stack address for this hart into virtual address */
    li t1, 0xffffffff00000000
    add sp, sp, t1

    /* Jump to rust entry function */
    lui t0, %hi(_start_rust)
    addi t0, t0, %lo(_start_rust)
    jr t0

_start_abort:
    wfi
    j _start_abort

    # 初始内核映射所用的页表
    .section .data
    .align 12
    .global boot_page_table
boot_page_table:
    .quad 0
    .quad 0
    # 第 2 项：0x8000_0000 -> 0x8000_0000，0xcf 表示 VRWXAD 均为 1
    .quad (0x80000 << 10) | 0xcf
    .zero 505 * 8
    # 第 508 项：0xffff_ffff_0000_0000 -> 0x0000_0000，0xcf 表示 VRWXAD 均为 1
    .quad (0x00000 << 10) | 0xcf
    .quad 0
    # 第 510 项：0xffff_ffff_8000_0000 -> 0x8000_0000，0xcf 表示 VRWXAD 均为 1
    .quad (0x80000 << 10) | 0xcf
    .quad 0
"#
);

#[doc(hidden)]
#[no_mangle]
#[rustfmt::skip]
pub unsafe extern "Rust" fn default_pre_init() {}

// by default, other harts other than hart zero won't be started.
// if you need to start these cores, redefine your `_mp_hook` function.
#[doc(hidden)]
#[no_mangle]
#[rustfmt::skip]
pub unsafe extern "Rust" fn default_mp_hook(hartid: usize, _dtb: usize) -> bool {
    match hartid {
        0 => true,
        _ => loop {
            riscv::asm::wfi()
        }, 
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    halt();
}

#[no_mangle]
extern "C" fn abort() -> ! {
    panic!("abort!");
}

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
    .equ REGBYTES, 8
    .macro SAVE reg, offset
        sd  \\reg, \\offset*REGBYTES(sp)
    .endm
    .macro LOAD reg, offset
        ld  \\reg, \\offset*REGBYTES(sp)
    .endm
"
);
#[cfg(target_pointer_width = "32")]
global_asm!(
    "
    .equ REGBYTES, 4
    .macro SAVE reg, offset
        sw  \\reg, \\offset*REGBYTES(sp)
    .endm
    .macro LOAD reg, offset
        lw  \\reg, \\offset*REGBYTES(sp)
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
    csrrw   sp, sscratch, sp
    bnez    sp, _trap_save_from_user
_trap_save_from_kernel:
    csrr    sp, sscratch
_trap_save_from_user:
    # 在栈上开辟 Context 所需的空间
    addi    sp, sp, -34*REGBYTES
    # 保存通用寄存器，除了 x0（固定为 0）
    SAVE    x1, 1
    addi    x1, sp, 34*REGBYTES
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
    mv      a0, sp
    csrr    a1, scause
    csrr    a2, stval
    jal     _start_trap_rust

    .globl __restore
# 离开中断
# 从 Context 中恢复所有寄存器，并跳转至 Context 中 sepc 的位置
__restore:
    /* 多线程环境下恢复上下文 */
    mv      sp, a0

    # 恢复 CSR
    LOAD    t0, 32
    LOAD    t1, 33
    # 不恢复 scause 和 stval
    csrw    sstatus, t0
    csrw    sepc, t1

    # 根据即将恢复的线程属于用户还是内核，恢复 sscratch
    # 检查 sstatus 上的 SPP 标记
    andi    t0, t0, 1 << 8
    bnez    t0, _trap_load_to_kernel
_trap_load_to_user:
    # 将要进入用户态，需要将内核栈地址写入 sscratch
    addi    t0, sp, 36*REGBYTES
    csrw    sscratch, t0
_trap_load_to_kernel:
    # 如果要进入内核态，sscratch 保持为 0 不变
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
