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

/* 目标架构 */
OUTPUT_ARCH(riscv)

/* 执行入口 */
ENTRY(_start)

SECTIONS
{
    /* .text 字段 */
    .text : {
        /* todo: stext, etext */
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

    /* .data 字段 */
    .data : {
        _sidata = LOADADDR(.data);
        _sdata = .;
        /* 要链接的文件的 .data 字段集中放在这里 */
        *(.sdata .sdata.* .sdata2 .sdata2.*);
        *(.data .data.*)
        _edata = .;
    } > REGION_DATA

    /* .bss 字段 */
    .bss (NOLOAD) : {
        _sbss = .;
        /* 要链接的文件的 .bss 字段集中放在这里 */
        *(.sbss .bss .bss.*)
        _ebss = .;
    } > REGION_BSS

    /* Discard .eh_frame, we are not doing unwind on panic so it is not needed */
    /DISCARD/ :
    {
        *(.eh_frame);
    }
}

