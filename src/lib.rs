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
pub unsafe extern "C" fn start_rust(hartid: usize, dtb_pa: usize) -> ! {
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
        fn _mp_hook(hartid: usize, dtb_pa: usize) -> bool;

        // entry function by supervisor implementation
        fn main(hartid: usize, dtb_pa: usize);
    }

    static READY: AtomicBool = AtomicBool::new(false);
    if _mp_hook(hartid, dtb_pa) {
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
    main(hartid, dtb_pa);

    // Shutdown
    riscv_sbi::legacy::shutdown()
}

#[cfg(target_pointer_width = "32")]
global_asm!(
    r#"
    .section .text.entry
    .globl _start
_start:
    /* Todo: paging system for 32-bit environment */

    lui ra, %hi(_abs_start)
    jr %lo(_abs_start)(ra)
"#
);

#[cfg(target_pointer_width = "64")]
global_asm!(
    r#"
    .section .text.entry
    .globl _start
_start:
    /* On system boot, we use Sv39 paging system. After boot succeeded, 
        operating system may switch to other paging systems. */

    /* load address of boot_page_table and calculate its page number */
    la t0, boot_page_table
    srli t0, t0, 12
    li t1, (8 << 60)    /* Use Sv39 in satp register */
    or t0, t0, t1
    /* Write to satp and refresh TLB */
    csrw satp, t0       
    sfence.vma          

    /* Jump to actual virtual start address */
.option push
.option norelax /* to prevent an unsupported R_RISCV_ALIGN relocation from being generated */
1:
    auipc ra, %pcrel_hi(1f)
    ld ra, %pcrel_lo(1b)(ra)
    jr ra
    .align  3
1:
    .dword _abs_start
.option pop

    /* Boot page table (initial kernal mapping). Can be recycled afterwards */
    .section .data
    .align 12
    .global boot_page_table
    boot_page_table:
    .quad 0
    .quad 0
    .quad (0x80000 << 10) | 0xcf /* Item 2: 0x0000000_80000000 -> 0x80000000 + VRWXAD (0xCF) */
    .zero 505 * 8
    .quad (0x00000 << 10) | 0xcf /* Item 508: 0xffffffff_00000000 -> 0x00000000 + VRWXAD (0xCF) */
    .quad 0
    .quad (0x80000 << 10) | 0xcf /* Item 510: 0xffffffff_80000000 -> 0x80000000 + VRWXAD (0xCF) */
    .quad 0
"#
);

global_asm!(
    r#"
_abs_start:
    .cfi_startproc
    .cfi_undefined ra

    /* a0: hart id */
    /* a1: device tree root */

    /* Setup global pointer */
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop

    /* Check hart id limit */
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

    /* If entry function returns, it should abort */
    la ra, _start_abort
    
    /* Jump to rust entry function */
    j _start_rust

    .cfi_endproc

_start_abort:
    wfi
    j _start_abort
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
pub unsafe extern "Rust" fn default_mp_hook(hartid: usize, _dtb_pa: usize) -> bool {
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
# 保存 Context 并且进入 rust 中的中断处理函数
_start_trap_sbi:
    # 因为线程当前的栈不一定可用，必须切换到内核栈来保存 Context 并进行中断流程
    # 因此，我们使用 sscratch 寄存器保存内核栈地址
    # 思考：sscratch 的值最初是在什么地方写入的？

    # 交换 sp 和 sscratch（切换到内核栈）
    csrrw   sp, sscratch, sp
    # 在内核栈开辟 Context 的空间
    addi    sp, sp, -36*REGBYTES

    # 保存通用寄存器，除了 x0（固定为 0）
    SAVE    x1, 1
    # 将本来的栈地址 sp（即 x2）保存
    csrr    x1, sscratch
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
    csrr    t0, sstatus
    csrr    t1, sepc
    SAVE    t0, 32
    SAVE    t1, 33
    # 调用 handle_interrupt，传入参数
    # context: &mut Context
    mv      a0, sp
    # scause: Scause
    csrr    a1, scause
    # stval: usize
    csrr    a2, stval
    jal _start_trap_rust

    .globl __restore
    # 离开中断
    # 此时内核栈顶被推入了一个 Context，而 a0 指向它
    # 接下来从 Context 中恢复所有寄存器，并将 Context 出栈（用 sscratch 记录内核栈地址）
    # 最后跳转至恢复的 sepc 的位置
__restore:
    # 从 a0 中读取 sp
    # 思考：a0 是在哪里被赋值的？（有两种情况）
    mv      sp, a0
    # 恢复 CSR
    LOAD    t0, 32
    LOAD    t1, 33
    csrw    sstatus, t0
    csrw    sepc, t1
    # 将内核栈地址写入 sscratch
    addi    t0, sp, 36*REGBYTES
    csrw    sscratch, t0

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
#[allow(missing_docs)]
#[derive(Clone, Debug)]
#[repr(C)]
pub struct TrapFrame {
    pub zero: usize,
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
    pub sstatus: Sstatus,
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
pub unsafe fn start_trap_rust(
    trap_frame: *mut TrapFrame,
    scause: Scause,
    stval: usize,
) -> *mut TrapFrame {
    extern "Rust" {
        fn ExceptionHandler(
            trap_frame: &mut TrapFrame,
            scause: Scause,
            stval: usize,
        ) -> *mut TrapFrame;
    }

    if scause.is_exception() {
        ExceptionHandler(&mut *trap_frame, scause, stval)
    } else {
        let code = scause.code();
        if code < __INTERRUPTS.len() {
            let h = &__INTERRUPTS[code];
            // if reserved, it would call DefaultHandler
            (h.handler)(&mut *trap_frame, scause, stval)
        } else {
            DefaultHandler();
            trap_frame
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
    handler: unsafe fn(trap_frame: &mut TrapFrame, scause: Scause, stval: usize) -> *mut TrapFrame,
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
    fn UserSoft(trap_frame: &mut TrapFrame, scause: Scause, stval: usize) -> *mut TrapFrame;
    fn SupervisorSoft(trap_frame: &mut TrapFrame, scause: Scause, stval: usize) -> *mut TrapFrame;
    fn UserTimer(trap_frame: &mut TrapFrame, scause: Scause, stval: usize) -> *mut TrapFrame;
    fn SupervisorTimer(trap_frame: &mut TrapFrame, scause: Scause, stval: usize) -> *mut TrapFrame;
    fn UserExternal(trap_frame: &mut TrapFrame, scause: Scause, stval: usize) -> *mut TrapFrame;
    fn SupervisorExternal(
        trap_frame: &mut TrapFrame,
        scause: Scause,
        stval: usize,
    ) -> *mut TrapFrame;

    fn DefaultHandler();
}
