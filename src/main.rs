#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

extern crate panic_semihosting;
use rtfm::app;
use rtfm::cyccnt::U32Ext;

use stm32h7xx_hal::gpio::{gpiob::PB0, gpiob::PB14, Output, PushPull};
//use stm32h7xx_hal::prelude::*;
use stm32h7xx_hal::flash::FlashExt;
use stm32h7xx_hal::gpio::GpioExt;
use stm32h7xx_hal::rcc::RccExt;
use stm32h7xx_hal::pwr::PwrExt;
use embedded_hal::digital::v2::OutputPin;

const PERIOD: u32 = 100_000_000;


// We need to pass monotonic = rtfm::cyccnt::CYCCNT to use schedule feature of RTFM
#[app(device = stm32h7xx_hal::pac, peripherals = true, monotonic = rtfm::cyccnt::CYCCNT)]
const APP: () = {
    // Global resources (global variables) are defined here and initialized with the
    // `LateResources` struct in init
    struct Resources {
        user_led1: PB0<Output<PushPull>>,
        user_led3: PB14<Output<PushPull>>,
    }

    #[init(schedule = [blinker])]
    fn init(cx: init::Context) -> init::LateResources {
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

//        let gpiob = device.GPIOB.split(&mut ccdr.ahb4);
//        let gpioe = device.GPIOE.split(&mut ccdr.ahb4);

//        let mut rcc = device.RCC.constrain();
//        let _clocks = rcc
//            .config
//            .use_hse(8.mhz())
//            .sysclk(480.mhz())
//            .pclk1(36.mhz())
//            .freeze(&mut flash.acr);

        // Setup LED
        let gpiob = device.GPIOB.split(&mut ccdr.ahb4);
        let mut led1 = gpiob.pb0.into_push_pull_output();
        led1.set_low().unwrap();
        let mut led3 = gpiob.pb14.into_push_pull_output();
        led3.set_low().unwrap();

        // Schedule the blinking task
        cx.schedule.blinker(cx.start + PERIOD.cycles()).unwrap();

        init::LateResources {
            user_led1: led1,
            user_led3: led3,
        }
    }

    #[task(resources = [user_led1, user_led3], schedule = [blinker])]
    fn blinker(cx: blinker::Context) {
        // Use the safe local `static mut` of RTFM
        static mut LED_STATE: bool = false;

        if *LED_STATE {
            cx.resources.user_led1.set_high().unwrap();
            cx.resources.user_led3.set_high().unwrap();
            *LED_STATE = false;
        } else {
            cx.resources.user_led1.set_low().unwrap();
            cx.resources.user_led3.set_low().unwrap();
            *LED_STATE = true;
        }
        cx.schedule.blinker(cx.scheduled + PERIOD.cycles()).unwrap();
    }

    // needed for dispatching tasks??
    extern "C" {
        fn EXTI0();
    }
};
