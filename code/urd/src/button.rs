// This module defines a button struct and method to make it easier to interface with

use crate::hal::gpio::{Pin, Input};
use crate::{helpers::ms_to_cycles, CLOCK_MHZ};

pub struct Button<const P: char, const N: u8, M> {
    pub pin: Pin<P, N, Input<M>>, // Button pin

    pub press_raw: bool, // true if the switch pin is high
    pub press_start_cycle: Option<u64>, // Processor cycles elapsed when the button started being pressed


    pub long_press_cycles: u64, // Cycles that must elapse between current clock cycle and press_start_cycle for a press to be considered a long press
    pub long_press: bool, // true if a long press has been registered

    pub last_press_cycle: u64, // Processor cycles elapsed when the button was last pressed
    pub debounce_cycles: u64, // Minimum number of processor cycles between button presses

    pub consecutive_cycles: u64, // After this many cycles have elapsed between the last button press and current button press the press is no longer sequential
    pub consecutive_presses: u32, // Number of presses that have been made in quick succesion
}

impl<const P: char, const N: u8, M> Button<P, N, M> {

    pub fn new(pin: Pin<P, N, Input<M>>) -> Self {
        Button {
            pin: pin,
            press_raw: false,
            press_start_cycle: None,
            long_press_cycles: ms_to_cycles(650, CLOCK_MHZ as u64), // Button needs to be held for atleast 650ms for a long press
            long_press: false,
            last_press_cycle: 0,
            debounce_cycles: ms_to_cycles(50, CLOCK_MHZ as u64), // 50ms debounce
            consecutive_cycles: ms_to_cycles(150, CLOCK_MHZ as u64), // When button presses are registered less than 150ms apart then the presses are sequential
            consecutive_presses: 0,
        }
    }

    // Returns true only when the button is pressed, so true will not be returned when the button is held down or bouncing
    // This functionality is dependant on the buttons debounce_cycles
    pub fn pressed(&mut self, clock_cycles: u64) -> bool {
        self.long_press = false;

        let mut pressed = false;
        let pin_high = self.pin.is_high();
        self.press_raw = pin_high;

        if pin_high {

            // Update press start cycle and check for long presses
            match self.press_start_cycle {
                Some(start_cycle) => {
                    self.long_press = clock_cycles - start_cycle >= self.long_press_cycles;
                },
                None => self.press_start_cycle = Some(clock_cycles),
            }
            
            // Set pressed to true if the button has been pressed and isn't bouncing
            if clock_cycles > (self.last_press_cycle + self.debounce_cycles) {
                pressed = true;
            }
            self.last_press_cycle = clock_cycles;
        } else {
            self.press_start_cycle = None; // When the button is not pressed there is no press start cycle
        }

        // Detect consecutive presses
        if (clock_cycles - self.last_press_cycle) < self.consecutive_cycles {
            if pressed {
                self.consecutive_presses += 1;
            }
        } else {
            self.consecutive_presses = 0;
        }

        pressed
    }
}