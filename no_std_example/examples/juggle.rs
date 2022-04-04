#![deny(unsafe_code)]
// #![deny(warnings)]
// #![allow(no_mangle)]
#![no_main]
#![no_std]

use panic_semihosting as _;

#[rtic::app(device = stm32f4::stm32f411, /*  dispatchers = [EXTI0]) */)]
mod app {

    use cortex_m::asm;
    use cortex_m::peripheral::DWT;
    use cortex_m_semihosting::hprintln;
    use fastmem::{Box, FastMem, FastMemStore};
    use stm32f4::stm32f411::interrupt;

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
        
        let fm: &FastMem<128, 128> = cx.local.fm_store.init();

        cx.core.DWT.enable_cycle_counter();
        cx.core.DWT.set_cycle_count(0);

        (Shared { fm }, Local {}, init::Monotonics())
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        // measure first alloc, u8
        rtic::pend(interrupt::EXTI1);
        hprintln!("idle --");
        // measure re-cycle
        rtic::pend(interrupt::EXTI1);
        hprintln!("idle --");

        // measure first alloc, u32
        rtic::pend(interrupt::EXTI2);
        hprintln!("idle --");
        // measure re-cycle
        rtic::pend(interrupt::EXTI2);
        hprintln!("idle --");

        // measure first alloc, u64
        rtic::pend(interrupt::EXTI3);
        hprintln!("idle --");
        // measure re-cycle
        rtic::pend(interrupt::EXTI3);
        hprintln!("idle --");

        rtic::pend(interrupt::EXTI0);

        loop {}
    }


   

    #[task( 
        binds = EXTI1,
        shared = [fm],
    )]
    fn t1(cx: t1::Context,) {
        let start = DWT::cycle_count();
        let n = cx.shared.fm.box_new(1u8);
        let end = DWT::cycle_count();
        hprintln!("t1 box {}", *n);
        hprintln!("t1 alloc {:?}", end.wrapping_sub(start));
        let start = DWT::cycle_count();
        drop(n);
        let end = DWT::cycle_count();
        hprintln!("t1 drop {}", end.wrapping_sub(start));
    }

    #[task( 
        binds = EXTI2,
        shared = [fm], 
    )]
    fn t2(cx: t2::Context,) {
        let start = DWT::cycle_count();
        let n = cx.shared.fm.box_new(1u32);
        let end = DWT::cycle_count();
        hprintln!("t2 box {}", *n);
        hprintln!("t2 alloc {:?}", end.wrapping_sub(start));
        let start = DWT::cycle_count();
        drop(n);
        let end = DWT::cycle_count();
        hprintln!("t2 drop {}", end.wrapping_sub(start));
    }

    #[task( 
        binds = EXTI3,
        shared = [fm], 
    )]
    fn t3(cx: t3::Context,) {
        let start = DWT::cycle_count();
        let n = cx.shared.fm.box_new(1u64);
        let end = DWT::cycle_count();
        hprintln!("t3 box {}", *n);
        
        hprintln!("t3 alloc {:?}", end.wrapping_sub(start));
        let start = DWT::cycle_count();
        drop(n);
        let end = DWT::cycle_count();
        hprintln!("t3 drop {}", end.wrapping_sub(start));
    }

    #[task( 
        binds = EXTI0,
        shared = [fm], 
    )]
    fn t0(cx: t0::Context,) {
        let start = DWT::cycle_count();
        let n = cx.shared.fm.box_new(1u64);
        let end = DWT::cycle_count();
        hprintln!("t0 box {}", *n);
        hprintln!("t0 alloc {:?}", end.wrapping_sub(start));

        let start = DWT::cycle_count();
        let n1 = cx.shared.fm.box_new(1u64);
        let end = DWT::cycle_count();
        hprintln!("t0 box {}", *n);
        hprintln!("t0 alloc {:?}", end.wrapping_sub(start));


        let start = DWT::cycle_count();
        drop(n);
        let end = DWT::cycle_count();
        hprintln!("t0 drop {}", end.wrapping_sub(start));

        let start = DWT::cycle_count();
        drop(n1);
        let end = DWT::cycle_count();
        hprintln!("t0 drop {}", end.wrapping_sub(start));
    }
}
