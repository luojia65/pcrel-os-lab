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
        auipc   t0, 0   /* t0: start paddr, must align to 4K or 2M */

        .option push
        .option norelax
    1:  auipc   t1, %pcrel_hi(1f)
        ld      t1, %pcrel_lo(1b)(t1)
        j       2f
        .align  3
    1:  .dword _stext
        .option pop
    2:      /* t1: start vaddr */

        /* 
            If vaddr align to 1G and paddr align to 2M, jump to _start_2M_aligned
            Or if vaddr align to 2M and paddr align to 4K, jump to _start_4K_aligned
            Else the program abort
        */

        li      t2, 1 * 1024 * 1024 * 1024 - 1  \n/* t2: 1G align mask */
        li      t3, 2 * 1024 * 1024 - 1         \n/* t3: 2M align mask */
        li      t4, 4 * 1024 - 1                \n/* t4: 4K align mask */
        and     t5, t0, t3                  \n/* paddr is 2M aligned */
        and     t6, t1, t2                  \n/* vaddr is 1G aligned */
        or      t6, t5, t6
        beqz    t6, _start_2M_aligned       \n/* check alignment to 2M */
        and     t5, t0, t4                  \n/* paddr is 4K aligned */
        and     t6, t1, t3                  \n/* vaddr is 2M aligned */
        or      t6, t5, t6
        beqz    t6, _start_4K_aligned       \n/* check alignment to 4K */
    1:  j       1b                          \n/* Must aligned to 4K, or abort */
    
    _start_2M_aligned: /* vaddr: 1G; paddr: 2M */

        /*  _start_free + 0  ..= _start_free + 4K:  boot_page_2_paddr 
            _start_free + 4K ..= _start_free + 8K:  boot_page_1_va_paddr 
            _start_free + 8K ..= _start_free + 12K: boot_page_1_pa_paddr */

        /* Load boot page for start_vaddr => start_paddr */
        
        la      t2, _start_free     
        li      t3, 4096
        add     t2, t2, t3          \n/* t2: current memory address boot_page_1_va_paddr */
        li      t3, 512 * 8         
        add     t3, t3, t2          \n/* t3: maximum memory address */
        srli    t4, t0, 2
        ori     t4, t4, 0x0F        \n/* t4: current pte entry value, vrwx */
        li      t5, (1 << (10 + 9)) \n/* t5: add to t4 to get next ppn */
    1:  sd      t4, 0(t2)           \n/* Store pte to memory */    
        addi    t2, t2, 8           \n/* Prepare for next loop */
        add     t4, t4, t5
        bltu    t2, t3, 1b          \n/* Jump to next loop */

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
        
        /* Adjust parameters and start */

        li      a2, 3 * 4096        \n/* a2: start of free space minus boot pages */

        j       _start_paging
    
    _start_4K_aligned: /* vaddr: 2M; paddr: 4K */

    /*  _start_free + 0   ..= _start_free + 4K:  boot_page_2_paddr 
        _start_free + 4K  ..= _start_free + 8K:  boot_page_1_va_paddr 
        _start_free + 8K  ..= _start_free + 12K: boot_page_1_pa_paddr 
        _start_free + 12K ..= _start_free + 16K: boot_page_0_va_paddr
        _start_free + 16K ..= _start_free + 20K: boot_page_0_pa_paddr */

    /* Load boot page for start_vaddr => start_paddr */
        
        la      t2, _start_free     
        li      t3, 4096 * 3
        add     t2, t2, t3          \n/* t2: current memory address boot_page_0_va_paddr */
        li      t3, 512 * 8         
        add     t3, t3, t2          \n/* t3: maximum memory address */
        srli    t4, t0, 2
        ori     t4, t4, 0x0F        \n/* t4: current pte entry value, vrwx */
        li      t5, (1 << 10)       \n/* t5: add to t4 to get next ppn */
    1:  sd      t4, 0(t2)           \n/* Store pte to memory */    
        addi    t2, t2, 8           \n/* Prepare for next loop */
        add     t4, t4, t5
        bltu    t2, t3, 1b          \n/* Jump to next loop */

        la      t2, _start_free     
        li      t3, 4096
        add     t2, t2, t3          \n/* t2: boot_page_1_va_paddr */
        srli    t3, t1, 21
        andi    t3, t3, 0x1FF       \n/* t3: vpn1(start_vaddr) */
        slli    t4, t3, 3           \n/* t4: vpn1 * 8 */
        add     t5, t4, t2          \n/* t5: boot_page_1[vpn2] */
        la      t6, _start_free     
        li      s0, 4096 * 3
        add     t6, t6, s0          \n/* t6: boot_page_0_va_paddr */
        srli    t6, t6, 2
        ori     t6, t6, 0x01        \n/* t6: pte entry value, ->boot_page_1, v, leaf */
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
        li      t3, 4096 * 4
        add     t2, t2, t3          \n/* t2: boot_page_0_pa_paddr */
        srli    t3, t0, 12
        andi    t3, t3, 0x1FF       \n/* t3: vpn0 */
        slli    t4, t3, 3           \n/* t4: vpn0 * 8 */
        add     t5, t4, t2          \n/* t5: boot_page_0[vpn1] */
        srli    t6, t0, 2
        ori     t6, t6, 0x0F        \n/* t6: pte entry value, vrwx */
        sd      t6, 0(t5)
        
        la      t2, _start_free     
        li      t3, 4096 * 2
        add     t2, t2, t3          \n/* t2: boot_page_1_pa_paddr */
        srli    t3, t0, 21
        andi    t3, t3, 0x1FF       \n/* t3: vpn1 */
        slli    t4, t3, 3           \n/* t4: vpn1 * 8 */
        add     t5, t4, t2          \n/* t5: boot_page_1[vpn1] */
        la      t6, _start_free     
        li      s0, 4096 * 4
        add     t6, t6, s0          \n/* t6: boot_page_0_paddr */
        srli    t6, t6, 2
        ori     t6, t6, 0x01        \n/* t6: pte entry value, ->boot_page_0, v, leaf */
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

        /* Adjust parameters and start */

        li      a2, 5 * 4096        \n/* a2: start of free space minus boot pages */

    _start_paging:

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
