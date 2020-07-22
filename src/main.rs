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

#[export_name = "_start"]
#[link_section = ".init"] // this is stable
#[naked]
unsafe fn main() -> ! {
    asm!("
        auipc   t0, 0   /* t0: start paddr, must align to 2M */

        .option push
        .option norelax
    1:  auipc   t1, %pcrel_hi(1f)
        ld      t1, %pcrel_lo(1b)(t1)
        j       2f
        .align  3
    1:  .dword _stext
        .option pop
    2:      /* t1: start vaddr */

        li      t3, 2 * 1024 * 1024 - 1
        and     t2, t0, t3
        beqz    t2, _start_2M_aligned   /* check alignment to 2M */
        li      t5, 4 * 1024 - 1
        and     t4, t0, t5
        beqz    t4, _start_4K_aligned   /* check alignment to 4K */
    1:  j       1b                      /* Must aligned to 4K, or abort */
    
    _start_4K_aligned:  /* todo */
    _start_2M_aligned:

        /* Load boot page for start_vaddr => start_paddr */
        /*  _start_free + 0  ..= _start_free + 4K:  boot_page_2_paddr 
            _start_free + 4K ..= _start_free + 8K:  boot_page_1_va_paddr 
            _start_free + 8K ..= _start_free + 12K: boot_page_1_pa_paddr */
        
        la      t2, _start_free     
        li      t3, 4096
        add     t2, t2, t3          \n/* t2: boot_page_1_va_paddr */
        srli    t3, t1, 21
        andi    t3, t3, 0x1FF       \n/* t3: vpn1 */
        slli    t4, t3, 3           \n/* t4: vpn1 * 8 */
        add     t5, t4, t2          \n/* t5: boot_page_1[vpn1] */
        srli    t6, t0, 2
        ori     t6, t6, 0x0F        \n/* t6: pte entry value, vrwx */
        sd      t6, 0(t5)

        la      t2, _start_free     \n/* t2: boot_page_2_paddr */
        srli    t3, t1, 30
        andi    t3, t3, 0x1FF       \n/* t3: vpn2(start_vaddr) */
        slli    t4, t3, 3           \n/* t4: vpn2 * 8 */
        add     t5, t4, t2          \n/* t5: boot_page_2[vpn2] */
        la      t6, _start_free     
        li      s0, 4096
        add     t6, t6, s0          \n/* t6: boot_page_1_va_paddr */
        srli    t6, t6, 2
        ori     t6, t6, 0x01        \n/* t6: pte entry value, ->boot_page_1, v, leaf */
        sd      t6, 0(t5)

        /* Load boot page for start_paddr => start_paddr */
        
        la      t2, _start_free     
        li      t3, 4096 * 2
        add     t2, t2, t3          \n/* t2: boot_page_1_pa_paddr */
        srli    t3, t0, 21
        andi    t3, t3, 0x1FF       \n/* t3: vpn1 */
        slli    t4, t3, 3           \n/* t4: vpn1 * 8 */
        add     t5, t4, t2          \n/* t5: boot_page_1[vpn1] */
        srli    t6, t0, 2
        ori     t6, t6, 0x0F        \n/* t6: pte entry value, vrwx */
        sd      t6, 0(t5)
        
        la      t2, _start_free     \n/* t2: boot_page_2_paddr */
        srli    t3, t0, 30
        andi    t3, t3, 0x1FF       \n/* t3: vpn2 */
        slli    t4, t3, 3           \n/* t4: vpn2 * 8 */
        add     t5, t4, t2          \n/* t5: boot_page_2[vpn2] */
        la      t6, _start_free     
        li      s0, 4096 * 2
        add     t6, t6, s0          \n/* t6: boot_page_1_paddr */
        srli    t6, t6, 2
        ori     t6, t6, 0x01        \n/* t6: pte entry value, ->boot_page_1, v, leaf */
        sd      t6, 0(t5)
        
        /* Write boot page address into satp and refresh */
        srli    t2, t2, 12          \n/* t2: boot_page_2_ppn */
        li      t3, 8 << 60         \n/* t3: mode (Sv39) */
        or      t4, t2, t3          \n/* t4: satp value */
        csrw    satp, t4            
        sfence.vma

        /* Jump to virtual address of _abs_start */
        .option push
        .option norelax
    1:  auipc ra, %pcrel_hi(1f)
        ld ra, %pcrel_lo(1b)(ra)
    2:  auipc sp, %pcrel_hi(2f)
        ld sp, %pcrel_lo(2b)(sp)
        jr ra
        .align  3
    1:  .dword _abs_start
    2:  .dword _sstack
        .option pop
    ", options(noreturn));
}

#[export_name = "_abs_start"]
#[naked]
fn abs_start() -> ! {
    loop {}
}
