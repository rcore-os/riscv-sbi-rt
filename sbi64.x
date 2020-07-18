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

/* Maximum hart id, can be defined by user */
/* Used to calculate stack size limit in runtime */
PROVIDE(_max_hart_id = 0);
/* Supervisor stack size for each hart; default to 2K per hart, can be redefined by user */
/* Used in initializing stack for each hart in runtime */
PROVIDE(_hart_stack_size = 2K);
/* Provide supervisor runtime heap size; must be times of 4K */
PROVIDE(_heap_size = 0);
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
        _etext = .;
    } > REGION_TEXT

    /* .rodata 字段 */
    .rodata : ALIGN(4K) {
        _srodata = .;
        /* 要链接的文件的 .rodata 字段集中放在这里 */
        *(.rodata .rodata.*)
        . = ALIGN(4K);
        _erodata = .;
    } > REGION_RODATA

    /* .data 字段 */
    .data : ALIGN(4K) { 
        _sidata = LOADADDR(.data);
        _sdata = .;
        /* Must be called __global_pointer$ for linker relaxations to work. */
        PROVIDE(__global_pointer$ = . + 0x800);
        /* 要链接的文件的 .data 字段集中放在这里 */
        *(.sdata .sdata.* .sdata2 .sdata2.*);
        *(.data .data.*)
        . = ALIGN(4K);
        _edata = .;
    } > REGION_DATA

    /* .bss 字段 */
    .bss (NOLOAD) : ALIGN(4K) {
        _sbss = .;
        /* 要链接的文件的 .bss 字段集中放在这里 */
        *(.sbss .bss .bss.*)
        . = ALIGN(4K);
        _ebss = .;
    } > REGION_BSS

    /* fictitious region that represents the memory available for the heap */
    .heap (NOLOAD) : ALIGN(4K) {
        _sheap = .;
        . += _heap_size;
        . = ALIGN(4K);
        _eheap = .;
    } > REGION_HEAP

    /* fictitious region that represents the memory available for the stack */
    .stack (INFO) : ALIGN(4K) {
        _estack = .;
        . = _stack_start;
        . = ALIGN(4K);
        _sstack = .;
    } > REGION_STACK

    /* Discard .eh_frame, we are not doing unwind on panic so it is not needed */
    /DISCARD/ :
    {
        *(.eh_frame .eh_frame_hdr);
    }
}

ASSERT(ORIGIN(REGION_TEXT) % 4K == 0, "
ERROR(riscv-sbi-rt): the start of the REGION_TEXT must be 4K-byte aligned");

ASSERT(ORIGIN(REGION_RODATA) % 4K == 0, "
ERROR(riscv-sbi-rt): the start of the REGION_RODATA must be 4K-byte aligned");

ASSERT(ORIGIN(REGION_DATA) % 4K == 0, "
ERROR(riscv-sbi-rt): the start of the REGION_DATA must be 4K-byte aligned");

ASSERT(ORIGIN(REGION_HEAP) % 4K == 0, "
ERROR(riscv-sbi-rt): the start of the REGION_HEAP must be 4K-byte aligned");

ASSERT(ORIGIN(REGION_STACK) % 4K == 0, "
ERROR(riscv-sbi-rt): the start of the REGION_STACK must be 4K-byte aligned");

ASSERT(_stext % 4K == 0, "
ERROR(riscv-sbi-rt): `_stext` must be 4K-byte aligned");

ASSERT(_sdata % 4K == 0 && _edata % 4K == 0, "
BUG(riscv-sbi-rt): .data is not 4K-byte aligned");

ASSERT(_sidata % 4K == 0, "
BUG(riscv-sbi-rt): the LMA of .data is not 4K-byte aligned");

ASSERT(_sbss % 4K == 0 && _ebss % 4K == 0, "
BUG(riscv-sbi-rt): .bss is not 4K-byte aligned");

ASSERT(_sheap % 4K == 0, "
BUG(riscv-sbi-rt): start of .heap is not 4K-byte aligned");

ASSERT(_stext + SIZEOF(.text) < ORIGIN(REGION_TEXT) + LENGTH(REGION_TEXT), "
ERROR(riscv-sbi-rt): The .text section must be placed inside the REGION_TEXT region.
Set _stext to an address smaller than 'ORIGIN(REGION_TEXT) + LENGTH(REGION_TEXT)'");

ASSERT(SIZEOF(.stack) > (_max_hart_id + 1) * _hart_stack_size, "
ERROR(riscv-rt): .stack section is too small for allocating stacks for all the harts.
Consider changing `_max_hart_id` or `_hart_stack_size`.");
