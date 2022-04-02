#![deny(unsafe_code)]
// #![deny(warnings)]
// #![allow(no_mangle)]
#![no_main]
#![no_std]

use panic_semihosting as _;

#[rtic::app(device = lm3s6965, peripherals = true)]
mod app {

    // use core::mem::MaybeUninit;
    use cortex_m::asm;
    use cortex_m::peripheral::DWT;
    use cortex_m_semihosting::hprintln;
    use fastmem::{Box, He, H};

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    type Heap = He<128, 128>;

    #[init(local = [
        heap: H<128, 128> = H::new(),
    ])]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        hprintln!("init");

        let heap = cx.local.heap.init();
        cx.core.DWT.enable_cycle_counter();
        cx.core.DWT.set_cycle_count(0);

        let start = DWT::cycle_count();
        asm::bkpt();
        let n_u8 = heap.box_new(1u8);
        asm::bkpt();
        let end = DWT::cycle_count();

        hprintln!("Diff {:?}", end.wrapping_sub(start));

        let start = DWT::cycle_count();
        asm::bkpt();
        let n_u32 = heap.box_new(32u32);
        asm::bkpt();
        let end = DWT::cycle_count();

        hprintln!("Diff {:?}", end.wrapping_sub(start));

        hprintln!("{}", *n_u8);
        hprintln!("{}", *n_u32);

        hprintln!("n_u8 @{:p}", &*n_u8);
        hprintln!("n_u32 @{:p}", &*n_u32);

        drop(n_u8); // force drop
        drop(n_u32); // force drop

        hprintln!("after dropping, re-cycled alloc");

        let start = DWT::cycle_count();
        asm::bkpt();
        let n_u8 = heap.box_new(1u8);
        asm::bkpt();
        let end = DWT::cycle_count();

        hprintln!("Diff {:?}", end.wrapping_sub(start));

        let start = DWT::cycle_count();
        asm::bkpt();
        let n_u32 = heap.box_new(32u32);
        asm::bkpt();
        let end = DWT::cycle_count();

        hprintln!("Diff {:?}", end.wrapping_sub(start));

        hprintln!("n_u8 @{:p}", &*n_u8);
        hprintln!("n_u32 @{:p}", &*n_u32);

        drop(n_u8); // force drop
        drop(n_u32); // force drop

        (Shared {}, Local {}, init::Monotonics())
    }
    #[inline(never)]
    pub fn alloc<T>(h: &'static Heap, v: T) -> Box<T> {
        h.box_new(v)
    }
}
