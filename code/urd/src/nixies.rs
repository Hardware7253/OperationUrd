// This module is for the transmission of data to the nixie tube shift registers

use stm32f1xx_hal as hal;
use hal::{
    gpio::{Pin, Output, PushPull, OpenDrain, Alternate},
    spi::*,
    timer::SysDelay,
    prelude::*
};

// All possible characters which can be displayed on the tubes
// Presented in the array in the order they are placed within the tube
pub const ALL_NIXIE_CHARACTERS: [char; 11] = ['1', '2', '6', '7', '5', '0', '4', '9', '8', '3', '.'];


// Struct contains pins and spi bus which are used to interface with the VFD display
pub struct Nixies<I: Instance, R, const P1: char, const P2: char, const C: u8, const D: u8, const L: u8, const O: u8> {
    pub spi_bus: Spi<I, R, (Pin<P1, C, Alternate>, NoMiso, Pin<P1, D, Alternate>), u8>, // Spi bus which the shift registers are attached to

    // Latch and output enable pin are on the same pin bank
    pub latch_pin: Pin<P2, L, Output<PushPull>>,
    pub oe_pin: Pin<P2, O, Output<OpenDrain>>, // OE pin should use an open drain because it has an external pullup resistor, OE is active low
}

// Represents which bit needs to be on (in the shift registers) to enable a nixie tube
pub const SELECT_BITS: [u8; 8] = [11, 10, 9, 8, 15, 14, 13, 12];

impl<I: Instance, R, const P1: char, const P2: char, const C: u8, const D: u8, const L: u8, const O: u8> Nixies<I, R, P1, P2, C, D, L, O> {

    // Writes bytes over the spi interface to the VFD shift registers
    fn spi_write(&mut self, words: &[u8], delay: &mut SysDelay) {
        let _ = self.spi_bus.write(words);

        // Latch data
        delay.delay_us(1u32);
        self.latch_pin.set_high();
        delay.delay_us(1u32);
        self.latch_pin.set_low();
    }

    // Turns off all nixie tubes
    pub fn turn_off(&mut self, delay: &mut SysDelay) {
        self.spi_write(&[0, 0, 0], delay);
    }

    // Writes a character to the specified nixie tube
    pub fn write_char(&mut self, character: char, nixie: usize, delay: &mut SysDelay) {

        // Get the bit which needs to be turned on inorder to display a matching character on the selected nixie tube
        let shift: usize = match character {
            '0' => 7,
            '1' => 18,
            '2' => 19,
            '3' => 20,
            '4' => 21,
            '5' => 22,
            '6' => 3,
            '7' => 4,
            '8' => 5,
            '9' => 6,
            '.' => 17,
            _ => 0,
        };

        // The only bits on in write_num are the shift bit and the nixie select bit
        let write_num: u32 = 1 << shift | 1 << SELECT_BITS[nixie];

        // Turn write num into 3 bytes to send over the spi interface
        // Send most significant bits first
        let write_words: &[u8] = &[
            (write_num >> 16) as u8,
            (write_num >> 8) as u8,
            write_num as u8
        ];

        self.spi_write(write_words, delay);
    }

    // Display a string slice on nixie tubes
    pub fn display_str(&mut self, string_slice: &str, delay: &mut SysDelay) {
        for (i, character) in string_slice.chars().enumerate() {
            self.write_char(character, i, delay);
            delay.delay_ms(1u16);
        }
    }
}