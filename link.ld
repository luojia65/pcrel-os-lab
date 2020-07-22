MEMORY {
    /* 存储单元的物理地址 */
    SRAM : ORIGIN = 0xffffffff80000000, LENGTH = 128M
}

PROVIDE(_stext = 0xffffffff80000000); /* 本次实现中虚拟地址必须对齐到2M，因为这部分只有一级页表 */
PROVIDE(_heap_size = 128K);
PROVIDE(_stack_size = 64K);

REGION_ALIAS("REGION_TEXT", SRAM);
REGION_ALIAS("REGION_RODATA", SRAM);
REGION_ALIAS("REGION_DATA", SRAM);
REGION_ALIAS("REGION_BSS", SRAM);
REGION_ALIAS("REGION_HEAP", SRAM);
REGION_ALIAS("REGION_STACK", SRAM);

SECTIONS {
    .text _stext : {
        /* Put reset handler first in .text section so it ends up as the entry */
        /* point of the program. */
        KEEP(*(.init));
        KEEP(*(.init.rust));
        . = ALIGN(4);
        (*(.trap));
        (*(.trap.rust));

        *(.text .text.*);
    } > REGION_TEXT

    .rodata : ALIGN(4) {
        *(.srodata .srodata.*);
        *(.rodata .rodata.*);

        /* 4-byte align the end (VMA) of this section.
          This is required by LLD to ensure the LMA of the following .data
          section will have the correct alignment. */
        . = ALIGN(4);
    } > REGION_RODATA

    .data : ALIGN(4) {
        _sidata = LOADADDR(.data);
        _sdata = .;
        /* Must be called __global_pointer$ for linker relaxations to work. */
        PROVIDE(__global_pointer$ = . + 0x800);
        *(.sdata .sdata.* .sdata2 .sdata2.*);
        *(.data .data.*);
        . = ALIGN(4);
        _edata = .;
    } > REGION_DATA AT > REGION_RODATA

    .bss (NOLOAD) : {
        _sbss = .;
        *(.sbss .sbss.* .bss .bss.*);
        . = ALIGN(4);
        _ebss = .;
    } > REGION_BSS

    /* fictitious region that represents the memory available for the heap */
    .heap (NOLOAD) : {
        _sheap = .;
        . += _heap_size;
        . = ALIGN(4);
        _eheap = .;
    } > REGION_HEAP

    /* fictitious region that represents the boot stack */
    .stack (NOLOAD) : {
        _estack = .;
        . += _stack_size;
        . = ALIGN(4);
        _sstack = .;
    } > REGION_STACK

    /* fake output .got section */
    /* Dynamic relocations are unsupported. This section is only used to detect
      relocatable code in the input files and raise an error if relocatable code
      is found */
    .got (INFO) : {
        KEEP(*(.got .got.*));
    }

    .eh_frame (INFO) : { KEEP(*(.eh_frame)) }
    .eh_frame_hdr (INFO) : { *(.eh_frame_hdr) }
}

ASSERT(_stext % 2M == 0, "virtual address must align to 2M or 1G")

SECTIONS {
    .free (NOLOAD) : ALIGN(4K) {
        _start_free = .;
    } > SRAM
}
