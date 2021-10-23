//! Helper functions for delays.

#![allow(dead_code)]

use crate::{micros, millis};

/// Sleep for a number of milliseconds.
pub fn delay_ms(value: u32) {
    let start = millis();
    while millis() < start + value {}
}

/// Sleep for a number of microseconds.
pub fn delay_us(value: u32) {
    let start = micros();
    while micros() < start + value as u64 {}
}
