#![deny(unsafe_code)]
// #![deny(warnings)]
// #![allow(no_mangle)]
#![no_main]
#![no_std]

use panic_semihosting as _;

#[rtic::app(device = stm32f4::stm32f411, dispatchers = [EXTI0])]
mod app {

    use cortex_m::asm;
    use cortex_m::peripheral::DWT;
    use cortex_m_semihosting::hprintln;
    use fastmem::{Box, FastMem, FastMemStore};

    #[shared]
    struct Shared {
        #[lock_free]
        fm: &'static FM,
    }

    #[local]
    struct Local {}

    type FM = FastMem<128, 128>;

    #[init(local = [
        fm_store: FastMemStore<128,128> = FastMemStore::new(),
    ])]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        hprintln!("init");
        asm::bkpt();
        let fm: &FastMem<128, 128> = cx.local.fm_store.init();

        cx.core.DWT.enable_cycle_counter();
        cx.core.DWT.set_cycle_count(0);

        let start = DWT::cycle_count();
        let b8 = fm.box_new(8u8);
        let end = DWT::cycle_count();
        hprintln!("Diff {:?}", end.wrapping_sub(start));
        hprintln!("box b {}", *b8);

        let start = DWT::cycle_count();
        let b32 = fm.box_new(32u32);
        let end = DWT::cycle_count();
        hprintln!("Diff {:?}", end.wrapping_sub(start));

        hprintln!("box b2 {}", *b32);

        t2::spawn(b8);
        t1::spawn();
        (Shared { fm }, Local {}, init::Monotonics())
    }

    #[task(shared = [fm])]
    fn t1(cx: t1::Context) {
        hprintln!("t1");
        let start = DWT::cycle_count();
        asm::bkpt();
        let n_u8 = cx.shared.fm.box_new(1u8);
        asm::bkpt();
        let end = DWT::cycle_count();
        hprintln!("t1 box {}", *n_u8);

        hprintln!("t1 Diff {:?}", end.wrapping_sub(start));
        t2::spawn(n_u8);
    }

    #[task(shared = [])]
    fn t2(cx: t2::Context, b: Box<u8>) {
        let start = DWT::cycle_count();
        asm::bkpt();
        hprintln!("t2 box {}", *b);
        drop(b)
    }
    // #[inline(never)]
    // pub fn alloc<T>(h: &'static FM, v: T) -> Box<T> {
    //     h.box_new(v)
    // }
}
