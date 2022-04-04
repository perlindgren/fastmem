#![deny(unsafe_code)]
// #![deny(warnings)]
// #![allow(no_mangle)]
#![no_main]
#![no_std]

use panic_semihosting as _;

#[rtic::app(device = stm32f4::stm32f411, dispatchers = [EXTI0])]
mod app {

    // use core::mem::MaybeUninit;
    use cortex_m::peripheral::DWT;
    use cortex_m_semihosting::hprintln;
    use fastmem::{FastMem, FastMemStore};

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init(local = [
        fm_store: FastMemStore<128,128> = FastMemStore::new(),
    ])]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        hprintln!("init");

        let fm: &FastMem<128, 128> = cx.local.fm_store.init();
        cx.core.DWT.enable_cycle_counter();
        cx.core.DWT.set_cycle_count(0);

        let start = DWT::cycle_count();
        let n_u8 = fm.box_new(1u8);
        let end = DWT::cycle_count();

        hprintln!("Diff {:?}", end.wrapping_sub(start));

        let start = DWT::cycle_count();
        let n_u32 = fm.box_new(32u32);
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
        let n_u8 = fm.box_new(1u8);
        let end = DWT::cycle_count();

        hprintln!("Diff {:?}", end.wrapping_sub(start));

        let start = DWT::cycle_count();
        let n_u32 = fm.box_new(32u32);
        let end = DWT::cycle_count();

        hprintln!("Diff {:?}", end.wrapping_sub(start));

        hprintln!("n_u8 @{:p}", &*n_u8);
        hprintln!("n_u32 @{:p}", &*n_u32);

        drop(n_u8); // force drop
        drop(n_u32); // force drop

        (Shared {}, Local {}, init::Monotonics())
    }
}
