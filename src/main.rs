#![no_std]
#![no_main]

#![feature(naked_functions)]
#![feature(alloc_error_handler)]
#![feature(asm)]

use core::alloc::Layout;
use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[alloc_error_handler]
fn oom(_layout: Layout) -> ! {
    loop {}
}

#[repr(align(4096))]
struct __Page([usize; 512]);

#[export_name = "_boot_page_2"]
static mut __BOOT_PAGE_2: __Page = __Page([0; 512]);
#[export_name = "_boot_page_1"]
static mut __BOOT_PAGE_1: __Page = __Page([0; 512]);
#[export_name = "_boot_page_0"]
static mut __BOOT_PAGE_0: __Page = __Page([0; 512]);

#[export_name = "_start"]
#[link_section = ".init"] // this is stable
#[naked]
unsafe fn main() -> ! {
    let start_paddr: usize;
    asm!("
        auipc   {start_paddr}, 0
        la      sp, _sstack
        andi    {tmp}, {start_paddr}, -1 /* 0xFFF */
    1:  beqz    {tmp}, 1b
        li      {mask}, (0xFF << 56)
        and     {sext}, {start_paddr}, {mask}
        li      {high_bit}, (1 << 55)
        and     {high_bit}, {start_paddr}, {high_bit}
        beqz    {high_bit}, 2f
    1:  bne     {sext}, {mask}, 1b
        j       3f
    2:  bnez    {sext}, 2b
    3:
    ", 
        start_paddr = out(reg) start_paddr,
        tmp = lateout(reg) _,
        mask = lateout(reg) _,
        sext = lateout(reg) _,
        high_bit = lateout(reg) _,
    );
    extern {
        static _stext: u8;
    }
    let start_vaddr = &_stext as *const _ as usize;
    // 0xffffffff80000000 => start_paddr
    let vpn2 = 510;
    __BOOT_PAGE_2.0[vpn2] = (start_paddr >> 2) | 0x0f; // vrwx
    // start_paddr (start_vaddr = start_paddr) => start_paddr
    let (vpn2, vpn1, vpn0) = (
        (start_paddr >> 30) & 0x1FF, 
        (start_paddr >> 21) & 0x1FF, 
        (start_paddr >> 12) & 0x1FF, 
    );
    __BOOT_PAGE_0.0[vpn0] = (start_paddr >> 2) | 0x0f;
    let page0_vaddr = &__BOOT_PAGE_0 as *const _ as usize;
    let page0_paddr = page0_vaddr - start_vaddr + start_paddr;
    __BOOT_PAGE_1.0[vpn1] = (page0_paddr >> 2) | 0x01; // for leaf
    let page1_vaddr = &__BOOT_PAGE_1 as *const _ as usize;
    let page1_paddr = page1_vaddr - start_vaddr + start_paddr;
    __BOOT_PAGE_2.0[vpn2] = (page1_paddr >> 2) | 0x01; // for leaf
    let page2_vaddr = &__BOOT_PAGE_2 as *const _ as usize;
    let page2_paddr = page2_vaddr - start_vaddr + start_paddr;
    asm!("
        srli    {satp}, {satp}, 12
        li      {mode}, 8 << 60
        or      {satp}, {satp}, {mode}
        csrw    satp, {satp}
        sfence.vma

        .option push
        .option norelax
    1:
        auipc ra, %pcrel_hi(1f)
        ld ra, %pcrel_lo(1b)(ra)
        jr ra
        .align  3
    1:
        .dword _abs_start
    .option pop
    ", 
        satp = in(reg) page2_paddr,
        mode = out(reg) _,
    );
    loop {}
}

#[export_name = "_abs_start"]
#[naked]
fn abs_start() -> ! {
    loop {}
}
