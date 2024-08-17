// This program turns the led on and off, and prints the current state of the led

#![no_std]
#![no_main]

use panic_rtt_core::{self, rtt_init_print, rprintln};

use cortex_m_rt::entry;
use stm32f1xx_hal as hal;
use hal::{
    pac,
    i2c::{BlockingI2c, DutyCycle, Mode as Mode_i2c},
    spi::*,
    prelude::*
};

use ds323x::{DateTimeAccess, Ds323x, Rtcc, Timelike};

use arrform::{arrform, ArrForm};
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;

pub mod cycle_counter;
pub mod nixies;
pub mod button;
pub mod helpers;

use helpers::{add_with_rollover, ms_to_cycles};

const DIVERGENCE_NUMBERS: [&'static str; 21] = [
    // Alpha
    "0.337187",
    "0.409420",
    "0.409431",
    "0.456903",
    "0.456914",
    "0.523299",
    "0.523307",
    "0.571015",
    "0.571046",
    "0.571082",

    // Beta
    "1.053649",
    "1.055821",
    "1.064750",
    "1.064756",
    "1.129848",
    "1.129954",
    "1.130205",
    "1.130238",
    "1.130426",
    "1.143688",
    "1.382733",
];

const CLOCK_MHZ: u32 = 72;

const ANTI_POISON_INTERVAL_MS: u64 = 10 * 60 * 1000; // 10 Minutes between anti poison routines
const ANTI_POISON_DURATION_MS: u64 = 15 * 1000; // Anti poison routines last for 15 seconds
const ANIT_POISON_PATTERN: [usize; 8] = [3, 1, 4, 0, 6, 2, 7, 5]; // What character each display starts showing during the anti poison routine
//const ANIT_POISON_PATTERN: [usize; 8] = [0, 1, 2, 3, 4, 5, 6, 7]; // This pattern creates a wave

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let mut cp = cortex_m::Peripherals::take().unwrap(); // Core peripherals
    let dp = pac::Peripherals::take().unwrap(); // Device peripherals

    // Take ownership over the raw flash and rcc devices and convert them into the corresponding HAL structs
    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain(); 

    // Freeze the configuration of all the clocks in the system and store the frozen frequencies in `clocks`
    let clocks = rcc.cfgr
        // External oscillator
        .use_hse(8.MHz())

        // Bus and core clocks
        .hclk(CLOCK_MHZ.MHz())
        .sysclk(CLOCK_MHZ.MHz())

        // Peripheral clocks
        .pclk1(12.MHz())
        .pclk2(12.MHz())
    .freeze(&mut flash.acr);
    
    let mut delay = cp.SYST.delay(&clocks);

    // Enable cycle counter
    cp.DCB.enable_trace();
    cp.DWT.enable_cycle_counter();

    let mut gpioa = dp.GPIOA.split();
    let mut gpiob = dp.GPIOB.split();
    


    // Construct rtc using i2c bus
    let scl = gpiob.pb10.into_alternate_open_drain(&mut gpiob.crh);
    let sda = gpiob.pb11.into_alternate_open_drain(&mut gpiob.crh);

    let i2c = BlockingI2c::i2c2(
        dp.I2C2,
        (scl, sda),
        Mode_i2c::Fast {
            frequency: 400.kHz(),
            duty_cycle: DutyCycle::Ratio16to9,
        },
        clocks,
        1000,
        10,
        1000,
        1000,
    );

    let mut rtc = Ds323x::new_ds3231(i2c);



    // Construct nixies using spi bus
    let spi_pins = (
        gpiob.pb13.into_alternate_push_pull(&mut gpiob.crh), // Clock
        NoMiso, // Miso
        gpiob.pb15.into_alternate_push_pull(&mut gpiob.crh)// Mosi
    );

    let spi_mode = Mode {
        polarity: Polarity::IdleLow,
        phase: Phase::CaptureOnFirstTransition,
    };

    let spi = Spi::spi2(dp.SPI2, spi_pins, spi_mode, 100.kHz(), clocks);

    let mut nixies = nixies::Nixies {
        spi_bus: spi,
        latch_pin: gpiob.pb12.into_push_pull_output(&mut gpiob.crh),
        oe_pin: gpiob.pb14.into_open_drain_output(&mut gpiob.crh),
    };

    nixies.turn_off(&mut delay);



    // Create a cycle counter struct to accurately keep track of elapsed clock cycles
    let mut cycle_counter = cycle_counter::Counter::new();

    // Setup switch pins has pulldown
    let on_switch_pin = gpioa.pa7.into_pull_down_input(&mut gpioa.crl);
    let mode_switch_pin = gpiob.pb0.into_pull_down_input(&mut gpiob.crl);

    // Buttons for adjusting the time
    let mut buttons = (
        button::Button::new(gpioa.pa3.into_pull_down_input(&mut gpioa.crl)), // Adjust hours
        button::Button::new(gpioa.pa4.into_pull_down_input(&mut gpioa.crl)), // Adjust minutes
        button::Button::new(gpioa.pa5.into_pull_down_input(&mut gpioa.crl)), // Adjust seconds
    );

    let mut time_adjust; // True when the users wants to adjust the clock time
    let mut can_exit_time_adjust = false;
    let mut divergence_index = 0; // Current index for the DIVERGENCE_NUMBERS constant

    // How many clock cyclces have to be elapsed before the anti-poison routine starts
    // This is reset after each anti-poison routine
    let mut activate_anti_posion_cycles = 0;

    loop {
        if on_switch_pin.is_high() { // Only do nixie stuff when the on switch is high
            cycle_counter.update();

            // When the cycle count exceeds activate_anti_posion_cycles then start the anti-poison routine
            if cycle_counter.cycles > activate_anti_posion_cycles {
                let deactivate_anti_posion_cycles = cycle_counter.cycles + ms_to_cycles(ANTI_POISON_DURATION_MS, CLOCK_MHZ as u64);

                // Randomise the divergence index
                let mut small_rng = SmallRng::seed_from_u64(cycle_counter.cycles);
                divergence_index = small_rng.gen_range(0..DIVERGENCE_NUMBERS.len());
                
                let mut increasing = [true; 8]; // Indicates wether the character index for each nixe is increasing or decreasing
                let mut current_indices = ANIT_POISON_PATTERN; // Current character index for each nixie

                // Anti poison routine
                // Lights each cathode in order
                while cycle_counter.cycles < deactivate_anti_posion_cycles {
                    cycle_counter.update();

                    // Update indices
                    for i in 0..nixies::SELECT_BITS.len() {
                        
                        // Increase ALL_NIXIE_CHARACTERS index for next iteration
                        let mut new_index = if increasing[i] {
                            current_indices[i] as i32 + 1
                        } else {
                            current_indices[i] as i32 - 1
                        };

                        if new_index < 0  {
                            new_index = 1;
                            increasing[i] = true;
                        }

                        if new_index > nixies::ALL_NIXIE_CHARACTERS.len() as i32 - 1 {
                            new_index = nixies::ALL_NIXIE_CHARACTERS.len() as i32 - 2;
                            increasing[i] = false;
                        }

                        current_indices[i] = new_index as usize;
                    }

                    // Display
                    for _ in 0..7 {
                        for i in 0..nixies::SELECT_BITS.len() {
                            nixies.write_char(nixies::ALL_NIXIE_CHARACTERS[current_indices[i]], i, &mut delay);
                            delay.delay_ms(1u32);
                        }
                    }
                }

                activate_anti_posion_cycles = cycle_counter.cycles + ms_to_cycles(ANTI_POISON_INTERVAL_MS, CLOCK_MHZ as u64); // Reset timer
            }

            if mode_switch_pin.is_high() {
                nixies.display_str(DIVERGENCE_NUMBERS[divergence_index], &mut delay); // Display divergence number
            } else {
                cycle_counter.update();

                // Update button structs
                buttons.0.pressed(cycle_counter.cycles);
                buttons.1.pressed(cycle_counter.cycles);
                buttons.2.pressed(cycle_counter.cycles);

                // Prevent time adjust mode being reactivated immedeately after leaving time adjust mode
                // Because the buttons would still all be registering a long press
                if (!buttons.0.press_raw || !buttons.1.press_raw || !buttons.2.press_raw) && can_exit_time_adjust {
                    can_exit_time_adjust = false;
                }

                // If all 3 buttons are held down activate time adjust mode
                time_adjust = buttons.0.long_press && buttons.1.long_press && buttons.2.long_press && !can_exit_time_adjust;


                
                // Read time from rtc
                let time = rtc.time().unwrap();
                let mut hour = time.hour();
                let mut minute = time.minute();
                let mut second = time.second();



                // Run time adjust mode in a while loop to prevent the rtc from incrementing the hours, minutes, or seconds while the user is trying to adjust.
                while time_adjust {
                    cycle_counter.update();

                    // Update button structs
                    let button_0_pressed = buttons.0.pressed(cycle_counter.cycles);
                    let button_1_pressed = buttons.1.pressed(cycle_counter.cycles);
                    let button_2_pressed = buttons.2.pressed(cycle_counter.cycles);

                    // Once any button is released it becomes possible for the user to exit time adjust mode
                    if (!buttons.0.press_raw || !buttons.1.press_raw || !buttons.2.press_raw) && !can_exit_time_adjust {
                        can_exit_time_adjust = true;
                    }

                    // If the user activates another long press exit time adjust mode
                    if buttons.0.long_press && buttons.1.long_press && buttons.2.long_press && can_exit_time_adjust {
                        time_adjust = false;
                    }

                    // Increment hours minutes and seconds when their respective buttons are pressed
                    if button_0_pressed && !buttons.1.press_raw && !buttons.2.press_raw {
                        hour = add_with_rollover(hour, 1, 0, 23);
                    }

                    if button_1_pressed && !buttons.2.press_raw && !buttons.0.press_raw {
                        minute = add_with_rollover(minute, 1, 0, 59);
                    }

                    if button_2_pressed && !buttons.0.press_raw && !buttons.1.press_raw {
                        second = add_with_rollover(second, 1, 0, 59);
                    }

                    // Update rtc datetime
                    if button_0_pressed || button_1_pressed || button_2_pressed {
                        let datetime = rtc.date()
                        .unwrap()
                        .and_hms_opt(hour, minute, second)
                        .unwrap();
                        rtc.set_datetime(&datetime).unwrap();
                    }

                    // Dsiplay adjusted time
                    let time_af = arrform!(64, "{}{} {}{} {}{}", 
                        hour / 10, hour % 10,
                        minute / 10, minute % 10,
                        second / 10, second % 10,
                    );

                    nixies.display_str(time_af.as_str(), &mut delay);
                }

                // Display time
                let time_af = arrform!(64, "{}{}.{}{}.{}{}", 
                        hour / 10, hour % 10,
                        minute / 10, minute % 10,
                        second / 10, second % 10,
                );

                nixies.display_str(time_af.as_str(), &mut delay);
            }
        } else {
            nixies.turn_off(&mut delay);
        }
    }
}
