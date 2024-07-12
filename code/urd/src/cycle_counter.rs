// This module contains code to help accurately keep track of the total clock cycles elapsed since since the board received power

use crate::hal::pac::DWT;

#[derive(Debug)]
pub struct Counter {
    pub cycles: u64, // Total cycle count
    pub cycle_resets: u32, // Number of times the DWT cycle count has rolled over
    pub last_cycle_count: u32, // Last DWT cycle count
}

impl Counter {
    pub fn new() -> Self {
        Counter {
            cycles: 0,
            cycle_resets: 0,
            last_cycle_count: 0,
        }
    }

    // Update cycle counter struct
    pub fn update(&mut self) {
        let dwt_cycles = DWT::cycle_count();

        // When the DWT cycle count resets increment cycle_resets
        if dwt_cycles < self.last_cycle_count {
            self.cycle_resets += 1; 
        }
        self.last_cycle_count = dwt_cycles;

        self.cycles = (self.cycle_resets as u64 * u32::MAX as u64) + dwt_cycles as u64; // Update cycle count
    }
}