PROVIDE(DefaultHandler = DefaultInterruptHandler);
PROVIDE(ExceptionHandler = DefaultExceptionHandler);

PROVIDE(UserSoft = DefaultHandler);
PROVIDE(SupervisorSoft = DefaultHandler);
PROVIDE(UserTimer = DefaultHandler);
PROVIDE(SupervisorTimer = DefaultHandler);
PROVIDE(UserExternal = DefaultHandler);
PROVIDE(SupervisorExternal = DefaultHandler);

PROVIDE(__pre_init = default_pre_init);
PROVIDE(_mp_hook = default_mp_hook);

/* Allow supervisor to redefine entry point address according to device */
PROVIDE(_stext = ORIGIN(REGION_TEXT));
/* Allow supervisor to redefine stack start according to device */
PROVIDE(_stack_start = ORIGIN(REGION_STACK) + LENGTH(REGION_STACK));

/* 目标架构 */
OUTPUT_ARCH(riscv)

/* 执行入口 */
ENTRY(_start)

SECTIONS
{
    /* .text 字段 */
    .text _stext : {
        /* 把 entry 函数放在最前面 */
        *(.text.entry)
        /* 要链接的文件的 .text 字段集中放在这里 */
        *(.text .text.*)
    } > REGION_TEXT

    /* .rodata 字段 */
    .rodata : {
        /* 要链接的文件的 .rodata 字段集中放在这里 */
        *(.rodata .rodata.*)
    } > REGION_RODATA

/* todo: align 4 bytes for XLEN=32, 8 bytes for XLEN=64 */
/* not required by riscv standard, but for higher effiency of rust's `r0` crate */

    /* .data 字段 */
    .data : ALIGN(4) { 
        _sidata = LOADADDR(.data);
        _sdata = .;
        /* 要链接的文件的 .data 字段集中放在这里 */
        *(.sdata .sdata.* .sdata2 .sdata2.*);
        *(.data .data.*)
        . = ALIGN(4);
        _edata = .;
    } > REGION_DATA

    /* .bss 字段 */
    .bss (NOLOAD) : ALIGN(4) {
        _sbss = .;
        /* 要链接的文件的 .bss 字段集中放在这里 */
        *(.sbss .bss .bss.*)
        . = ALIGN(4);
        _ebss = .;
    } > REGION_BSS

    /* fictitious region that represents the memory available for the stack */
    .stack (INFO) :
    {
        _estack = .;
        . = _stack_start;
        _sstack = .;
    } > REGION_STACK

    /* Discard .eh_frame, we are not doing unwind on panic so it is not needed */
    /DISCARD/ :
    {
        *(.eh_frame);
    }
}

ASSERT(ORIGIN(REGION_TEXT) % 4 == 0, "
ERROR(riscv-sbi-rt): the start of the REGION_TEXT must be 4-byte aligned");

ASSERT(ORIGIN(REGION_RODATA) % 4 == 0, "
ERROR(riscv-sbi-rt): the start of the REGION_RODATA must be 4-byte aligned");

ASSERT(ORIGIN(REGION_DATA) % 4 == 0, "
ERROR(riscv-sbi-rt): the start of the REGION_DATA must be 4-byte aligned");

ASSERT(ORIGIN(REGION_STACK) % 4 == 0, "
ERROR(riscv-rt): the start of the REGION_STACK must be 4-byte aligned");

ASSERT(_stext % 4 == 0, "
ERROR(riscv-sbi-rt): `_stext` must be 4-byte aligned");

ASSERT(_sdata % 4 == 0 && _edata % 4 == 0, "
BUG(riscv-sbi-rt): .data is not 4-byte aligned");

ASSERT(_sidata % 4 == 0, "
BUG(riscv-sbi-rt): the LMA of .data is not 4-byte aligned");

ASSERT(_sbss % 4 == 0 && _ebss % 4 == 0, "
BUG(riscv-sbi-rt): .bss is not 4-byte aligned");

ASSERT(_stext + SIZEOF(.text) < ORIGIN(REGION_TEXT) + LENGTH(REGION_TEXT), "
ERROR(riscv-sbi-rt): The .text section must be placed inside the REGION_TEXT region.
Set _stext to an address smaller than 'ORIGIN(REGION_TEXT) + LENGTH(REGION_TEXT)'");
