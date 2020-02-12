#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]



extern crate panic_itm;
//extern crate panic_semihosting;

use rtfm::app;
use rtfm::cyccnt::U32Ext;
use stm32h7xx_hal as processor_hal;

use processor_hal::gpio::{gpiob::PB0, gpiob::PB14, Output, PushPull, GpioExt};
use processor_hal::prelude::*;
use processor_hal::flash::FlashExt;
use processor_hal::rcc::RccExt;
use processor_hal::pwr::PwrExt;

//use processor_hal::i2c::{I2cExt};

use processor_hal::stm32 as pac;
use pac::I2C1;
use pac::DWT;

use embedded_hal::{
    digital::v2::{OutputPin, ToggleableOutputPin},
};

use cortex_m::{iprintln};
use cortex_m;

use bno080::*;

const BLINK_PERIOD: u32 = 1_000_000;
const IMU_READ_PERIOD: u32 = 10_000;

type ImuDriverType = bno080::BNO080<processor_hal::i2c::I2c<I2C1,
    (processor_hal::gpio::gpiob::PB8<processor_hal::gpio::Alternate<processor_hal::gpio::AF4>>,
     processor_hal::gpio::gpiob::PB9<processor_hal::gpio::Alternate<processor_hal::gpio::AF4>>)
>>;


// We need to pass monotonic = rtfm::cyccnt::CYCCNT to use schedule feature of RTFM
#[app(device = stm32h7xx_hal::stm32,  peripherals = true, monotonic = rtfm::cyccnt::CYCCNT)]
const APP: () = {
    // Global resources (global variables) are defined here and initialized with the
    // `LateResources` struct in init
    struct Resources {
        delay_source: processor_hal::delay::Delay,
        user_led1: PB0<Output<PushPull>>,
        user_led3: PB14<Output<PushPull>>,
        i2c1_driver: ImuDriverType,
        itm: cortex_m::peripheral::ITM,
    }

    /// First stage startup: interrupts are disabled
    #[init(spawn=[kicker], schedule=[blinker])]
    fn init(cx: init::Context) -> init::LateResources {
        // Note that interrupts are disabled in `init`

        // Enable cycle counter
        let mut core = cx.core;
        core.DWT.enable_cycle_counter();

        let dp: processor_hal::stm32::Peripherals = cx.device;
        let cp = cortex_m::Peripherals::take().unwrap();

        // Setup clocks
        let _flash = dp.FLASH.constrain();

        // Constrain and Freeze power
        let pwr = dp.PWR.constrain();
        let vos = pwr.freeze();

        // Constrain and Freeze clock
        let rcc = dp.RCC.constrain();

        //use the existing sysclk
        let mut ccdr = rcc.freeze(vos, &dp.SYSCFG);

        // source for delays
        let delay =  processor_hal::delay::Delay::new(cp.SYST, ccdr.clocks);

        // Setup LED
        let gpiob = dp.GPIOB.split(&mut ccdr.ahb4);
        let mut led1 = gpiob.pb0.into_push_pull_output();
        led1.set_high().unwrap();
        let mut led3 = gpiob.pb14.into_push_pull_output();
        led3.set_low().unwrap();

        // Schedule the blinking task
        cx.schedule.blinker(cx.start + BLINK_PERIOD.cycles()).unwrap();

        // setup the BNO080 imu device
        // On NUCLEO-H743ZI2 board, use pins 2 and 4 on CON7
        // (PB8 = I2C_1_SCL, PB9 = I2C_1_SDA)
        let scl = gpiob.pb8.into_alternate_af4().internal_pull_up(true).set_open_drain();
        let sda = gpiob.pb9.into_alternate_af4().internal_pull_up(true).set_open_drain();
        //let i2c_dev = dp.I2C1.i2c((scl, sda), 400.khz(), &ccdr);
        let i2c_dev =  processor_hal::i2c::I2c::i2c1(dp.I2C1, (scl, sda), 400.khz(), &ccdr);
        let i2c1_driver = BNO080::new(i2c_dev);
        
        cx.spawn.kicker().unwrap();

        init::LateResources {
            delay_source: delay,
            user_led1: led1,
            user_led3: led3,
            i2c1_driver: i2c1_driver,
            itm: cp.ITM,
        }

    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        // interrupts are enabled in `idle`
        loop {
            rtfm::export::wfi();
        }
    }

    /// Second phase startup: interrupts are enabled
    #[task(resources = [i2c1_driver, delay_source, itm], schedule = [imu_reader]) ]
    fn kicker(cx: kicker::Context) {
        iprintln!(&mut cx.resources.itm.stim[0], "| {} | kicker start", DWT::get_cycle_count() );
        let res =  cx.resources.i2c1_driver.init(cx.resources.delay_source);
        if res.is_err() {
            iprintln!( &mut cx.resources.itm.stim[0],"bno080 init err {:?}", res);
            return;
        }
        else {
            iprintln!(&mut cx.resources.itm.stim[0],"bno080 OK");
            cx.schedule.imu_reader(cx.scheduled + IMU_READ_PERIOD.cycles() ).unwrap();
        }

        //iprintln!(&mut cx.resources.itm.stim[0],"| {} | kicker done", DWT::get_cycle_count() );
    }

    #[task(resources = [i2c1_driver, user_led1, itm], schedule = [imu_reader])]
    fn imu_reader(cx: imu_reader::Context) {
        let handled_count = cx.resources.i2c1_driver.handle_all_messages();
        let sched_res = cx.schedule.imu_reader(cx.scheduled + IMU_READ_PERIOD.cycles());
        if sched_res.is_err() {
            iprintln!(&mut cx.resources.itm.stim[0],"imu sched err: {:?}", sched_res);
        }
        else {
            if handled_count > 0 {
                cx.resources.user_led1.toggle().unwrap();
                iprintln!(&mut cx.resources.itm.stim[0],"handled {} msgs", handled_count);
            }
        }
    }

    #[task(resources = [user_led1, user_led3, itm], schedule = [blinker])]
    fn blinker(cx: blinker::Context) {
        // Use the safe local `static mut` of RTFM
        static mut LED_STATE: bool = false;

        //iprintln!(&mut cx.resources.itm.stim[0], "| {} | blinker start", DWT::get_cycle_count() );

        if *LED_STATE {
            //cx.resources.user_led1.toggle().unwrap();
            cx.resources.user_led3.toggle().unwrap();
            *LED_STATE = false;
        }
        else {
            //cx.resources.user_led1.toggle().unwrap();
            cx.resources.user_led3.toggle().unwrap();
            *LED_STATE = true;
        }
        let sched_res = cx.schedule.blinker(cx.scheduled + BLINK_PERIOD.cycles());
        if sched_res.is_err() {
            iprintln!(&mut cx.resources.itm.stim[0],"blinkersched err: {:?}", sched_res);
        }

    }

    fn fatal_error_handler() {

    }
    // define a list of free/unused interrupts that rtfm may utilize
    // for dispatching software tasks
    extern "C" {
        //fn EXTI0();
        fn EXTI1();
        fn EXTI2();
        fn EXTI3();
    }
};





