//! Supervisor interrupt handler

// todo: should be put on crate `sbi` -> mod `interrupt`

// Ref: https://os20-rcore-tutorial.github.io/rCore-Tutorial-deploy/docs/lab-1/guide/part-6.html
// todo: should we save all registers here or part of them only?
global_asm!(
    "
#if __riscv_xlen == 64
# define STORE    sd
# define LOAD     ld
# define LOG_REGBYTES 3
#else
# define STORE    sw
# define LOAD     lw
# define LOG_REGBYTES 2
#endif
#define REGBYTES (1 << LOG_REGBYTES)

# 宏：将寄存器存到栈上
.macro SAVE reg, offset
    STORE  \\reg, \\offset*REGBYTES(sp)
.endm

# 宏：将寄存器从栈中取出
.macro RESTORE reg, offset
    LOAD  \\reg, \\offset*REGBYTES(sp)
.endm

    .section .text
    .globl _start_trap_sbi
    .align 2  # 对齐到4字节
# 进入中断
# 保存 Context 并且进入 rust 中的中断处理函数 interrupt::handler::handle_interrupt()
_start_trap_sbi:
    # 在栈上开辟 Context 所需的空间
    addi    sp, sp, -34*8
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
    RESTORE s1, 32
    RESTORE s2, 33
    # 不恢复 scause 和 stval
    csrw    sstatus, s1
    csrw    sepc, s2

    # 恢复通用寄存器
    RESTORE x1, 1
    RESTORE x3, 3
    RESTORE x4, 4
    RESTORE x5, 5
    RESTORE x6, 6
    RESTORE x7, 7
    RESTORE x8, 8
    RESTORE x9, 9
    RESTORE x10, 10
    RESTORE x11, 11
    RESTORE x12, 12
    RESTORE x13, 13
    RESTORE x14, 14
    RESTORE x15, 15
    RESTORE x16, 16
    RESTORE x17, 17
    RESTORE x18, 18
    RESTORE x19, 19
    RESTORE x20, 20
    RESTORE x21, 21
    RESTORE x22, 22
    RESTORE x23, 23
    RESTORE x24, 24
    RESTORE x25, 25
    RESTORE x26, 26
    RESTORE x27, 27
    RESTORE x28, 28
    RESTORE x29, 29
    RESTORE x30, 30
    RESTORE x31, 31

    # 恢复 sp（又名 x2）这里最后恢复是为了上面可以正常使用 RESTORE 宏
    RESTORE x2, 2
    sret
"
);

use riscv::register::{scause::Scause, stvec};

#[doc(hidden)]
pub fn setup_interrupts() {
    unsafe {
        extern "C" {
            /// `interrupt.asm` 中的中断入口
            fn _start_trap_sbi();
        }
        // 使用 Direct 模式，将中断入口设置为 `_start_trap_sbi`
        stvec::write(_start_trap_sbi as usize, stvec::TrapMode::Direct);
    }
}

/// Saved trap frame
pub struct TrapFrame {
    // todo: x1, x3, ...
}

#[doc(hidden)]
#[no_mangle]
#[allow(unused_variables, non_snake_case)]
pub fn DefaultExceptionHandler(trap_frame: &TrapFrame, scause: Scause, stval: usize) -> ! {
    crate::runtime::halt()
}

#[doc(hidden)]
#[no_mangle]
#[allow(unused_variables, non_snake_case)]
pub fn DefaultInterruptHandler() {
    crate::runtime::halt()
}

#[doc(hidden)]
#[export_name = "_start_trap_rust"]
pub fn start_trap_rust(trap_frame: *const TrapFrame, scause: Scause, stval: usize) {
    extern "Rust" {
        fn ExceptionHandler(trap_frame: &TrapFrame, scause: Scause, stval: usize);
    }

    unsafe {
        if scause.is_exception() {
            ExceptionHandler(&*trap_frame, scause, stval)
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
}

#[doc(hidden)]
pub union Vector {
    handler: unsafe fn(),
    reserved: unsafe fn(),
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
        handler: MachineSoft,
    },
    Vector { handler: UserTimer },
    Vector {
        handler: SupervisorTimer,
    },
    Vector {
        reserved: DefaultHandler,
    },
    Vector {
        handler: MachineTimer,
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
        handler: MachineExternal,
    },
];

extern "Rust" {
    fn UserSoft();
    fn SupervisorSoft();
    fn MachineSoft();
    fn UserTimer();
    fn SupervisorTimer();
    fn MachineTimer();
    fn UserExternal();
    fn SupervisorExternal();
    fn MachineExternal();

    fn DefaultHandler();
}
