#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]


//extern crate panic_itm;
extern crate panic_semihosting;

use rtfm::app;
use rtfm::cyccnt::U32Ext;

use stm32h7xx_hal::gpio::{gpiob::PB0, gpiob::PB14, Output, PushPull, GpioExt};
//use stm32h7xx_hal::prelude::*;
use stm32h7xx_hal::flash::FlashExt;
use stm32h7xx_hal::rcc::RccExt;
use stm32h7xx_hal::pwr::PwrExt;
use embedded_hal::digital::v2::OutputPin;

//#[cfg(debug_assertions)]
//use cortex_m_log::{print, println};
//use cortex_m_log::printer::semihosting;
use cortex_m_semihosting::{ hprintln};

//use cortex_m_log::{d_print, d_println};


const PERIOD: u32 = 100_000_000;


// We need to pass monotonic = rtfm::cyccnt::CYCCNT to use schedule feature of RTFM
#[app(device = stm32h7xx_hal::pac,  peripherals = true, monotonic = rtfm::cyccnt::CYCCNT)]
//#[app(device = stm32h7xx_hal::pac,  peripherals = true)]
const APP: () = {
    // Global resources (global variables) are defined here and initialized with the
    // `LateResources` struct in init
    struct Resources {
        user_led1: PB0<Output<PushPull>>,
        user_led3: PB14<Output<PushPull>>,
    }

    #[init(schedule = [blinker], spawn=[kicker])]
    fn init(cx: init::Context) -> init::LateResources {
        hprintln!("init").unwrap();

        // Enable cycle counter
        let mut core = cx.core;
        core.DWT.enable_cycle_counter();

        let device: stm32h7xx_hal::stm32::Peripherals = cx.device;

        // Setup clocks
        let _flash = device.FLASH.constrain();

        // Constrain and Freeze power
        let pwr = device.PWR.constrain();
        let vos = pwr.freeze();

        // Constrain and Freeze clock
        let rcc = device.RCC.constrain();
        //use the existing sysclk
        let mut ccdr = rcc.freeze(vos, &device.SYSCFG);

        // Setup LED
        let gpiob = device.GPIOB.split(&mut ccdr.ahb4);
        let mut led1 = gpiob.pb0.into_push_pull_output();
        led1.set_low().unwrap();
        let mut led3 = gpiob.pb14.into_push_pull_output();
        led3.set_low().unwrap();

        // Schedule the blinking task
        cx.schedule.blinker(cx.start + PERIOD.cycles()).unwrap();

        cx.spawn.kicker().unwrap();

        hprintln!("done with init").unwrap();

        //debug::exit(debug::EXIT_SUCCESS);

        init::LateResources {
            user_led1: led1,
            user_led3: led3,
        }

    }

//    #[init(spawn = [kicker])]
//    fn init(cx: init::Context)  {
//        hprintln!("init start").unwrap();
//        cx.spawn.kicker().unwrap();
//        hprintln!("init done").unwrap();
//    }

    #[task(spawn = [bar, baz])]
    fn kicker(cx: kicker::Context) {
        hprintln!("kicker start").unwrap();
        cx.spawn.bar().unwrap();
        cx.spawn.baz().unwrap();
        hprintln!("kicker done").unwrap();
    }

    #[task]
    fn bar(_: bar::Context) {
        hprintln!("bar start").unwrap();
        hprintln!("bar done").unwrap();
    }

    #[task]
    fn baz(_: baz::Context) {
        hprintln!("baz start").unwrap();
        hprintln!("baz done").unwrap();
    }

    #[task(resources = [user_led1, user_led3], schedule = [blinker])]
    fn blinker(cx: blinker::Context) {
        // Use the safe local `static mut` of RTFM
        static mut LED_STATE: bool = false;

        hprintln!("blinker").unwrap();

        if *LED_STATE {
            hprintln!(".").unwrap();
            cx.resources.user_led1.set_high().unwrap();
            cx.resources.user_led3.set_high().unwrap();
            *LED_STATE = false;
        }
        else {
            hprintln!("o").unwrap();
            cx.resources.user_led1.set_low().unwrap();
            cx.resources.user_led3.set_low().unwrap();
            *LED_STATE = true;
        }
        cx.schedule.blinker(cx.scheduled + PERIOD.cycles()).unwrap();
        //d_println!(log, "");

        hprintln!("ran").unwrap();
    }

    // needed for dispatching tasks??
    extern "C" {
        fn EXTI0();
        fn EXTI1();
        fn EXTI2();
        fn EXTI3();
    }
};
