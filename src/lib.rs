#![doc = include_str!("../README.md")]
#![no_std]
#![allow(dead_code)]

pub mod delay;

use core::cell::Cell;
use core::sync::atomic::{AtomicU32, Ordering};

use cortex_m::interrupt::{self, Mutex};
use cortex_m_rt::exception;

/// SysTick peripheral.
static SYSTICK: Mutex<Cell<Option<cortex_m::peripheral::SYST>>> = Mutex::new(Cell::new(None));

/// SysTick counter increased in interrupt.
static SYSTICK_COUNTER: AtomicU32 = AtomicU32::new(0);

/// System clock frequency in Hz.
static CLOCK_FREQ: AtomicU32 = AtomicU32::new(0);

/// SysTick frequency in Hz.
static TICK_FREQ: AtomicU32 = AtomicU32::new(0);

/// Optional callback function triggered within SysTick interrupt
static CALLBACK_FN: Mutex<Cell<Option<fn(u32)>>> = Mutex::new(Cell::new(None));

/// Initializes the SysTick counter with a frequency.
///
/// Sets the reload value according to the desired frequency and enables the interrupt.
/// Does not start the counter, use `start()` to accomplish.
/// - `syst` is the peripheral and will be consumed
/// - `clock_freq`: System core clock frequency in Hz
/// - `tick_freq`: SysTick frequency in Hz
pub fn init_with_frequency(mut syst: cortex_m::peripheral::SYST, clock_freq: u32, tick_freq: u32) {
    // Make sure interrupt does not run while doing the init
    syst.disable_interrupt();

    // Core clock must be used as source, otherwise calculations will be wrong
    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);

    // The tick counter should start with 0 after init
    SYSTICK_COUNTER.store(0, Ordering::Relaxed);

    // These values need to be stored for further calculations
    CLOCK_FREQ.store(clock_freq, Ordering::Relaxed);
    TICK_FREQ.store(tick_freq, Ordering::Relaxed);

    // Setup the timer registers with the required values
    let reload = (clock_freq / tick_freq) - 1;
    syst.set_reload(reload);
    syst.clear_current();

    // Finally start the interrupt and let everything run
    syst.enable_interrupt();

    interrupt::free(|cs| {
        SYSTICK.borrow(cs).replace(Some(syst));
    });
}

/// Returns the SysTick timer.
///
/// Use this function to get back ownership of the peripheral.
/// No prior actions like `stop()` are performed by this function.
pub fn free() -> cortex_m::peripheral::SYST {
    interrupt::free(|cs| SYSTICK.borrow(cs).replace(None).unwrap())
}

/// Starts the counter.
///
/// Initialisation must be done before calling this function.
/// Use `stop()` to halt the counter again.
pub fn start() {
    interrupt::free(|cs| {
        let mut systick = SYSTICK.borrow(cs).replace(None);
        systick.as_mut().unwrap().enable_counter();
        SYSTICK.borrow(cs).set(systick);
    })
}

/// Stops the counter.
pub fn stop() {
    interrupt::free(|cs| {
        let mut systick = SYSTICK.borrow(cs).replace(None);
        systick.as_mut().unwrap().disable_counter();
        SYSTICK.borrow(cs).set(systick);
    })
}

/// Resets the counter.
pub fn reset() {
    interrupt::free(|cs| {
        let mut systick = SYSTICK.borrow(cs).replace(None);
        systick.as_mut().unwrap().clear_current();
        SYSTICK.borrow(cs).set(systick);
        SYSTICK_COUNTER.store(0, Ordering::Relaxed);
    })
}

/// Returns the tick count.
pub fn ticks() -> u32 {
    SYSTICK_COUNTER.load(Ordering::Relaxed)
}

/// Returns the number of core clock cycles.
pub fn clock_cycles() -> u64 {
    interrupt::free(|cs| {
        let mut ticks = SYSTICK_COUNTER.load(Ordering::Relaxed);
        let mut systick = SYSTICK.borrow(cs).replace(None);
        let syst = systick.as_mut().unwrap();

        let load = syst.rvr.read();
        let val = syst.cvr.read();

        if syst.has_wrapped() {
            // This catches the case when the counter has reached 0 after
            // the last interrupt but before reading the value in the
            // statement above.
            ticks += 1;
        }

        SYSTICK.borrow(cs).set(systick);

        ((load as u64 + 1) * ticks as u64) + (load - val) as u64
    })
}

/// Returns elapsed milliseconds.
pub fn millis() -> u32 {
    (ticks() as u64 * 1000 / TICK_FREQ.load(Ordering::Relaxed) as u64) as u32
}

/// Returns elapsed microseconds.
pub fn micros() -> u64 {
    let sysclock_mhz = CLOCK_FREQ.load(Ordering::Relaxed) / 1000000;

    clock_cycles() / sysclock_mhz as u64
}

/// Set an interrupt callback function
///
/// The provided callback function is called on each SysTick interrupt
/// after updating the tick count and passed its value as argument
pub fn set_callback(callback: fn(u32)) {
    interrupt::free(|cs| {
        CALLBACK_FN.borrow(cs).set(Some(callback));
    });
}

/// Clear the interrupt callback function
pub fn clear_callback() {
    interrupt::free(|cs| {
        CALLBACK_FN.borrow(cs).set(None);
    });
}

#[exception]
#[allow(non_snake_case)]
fn SysTick() {
    interrupt::free(|cs| {
        // Increase the counter
        SYSTICK_COUNTER.fetch_add(1, Ordering::Relaxed);

        // Read the status register to ensure COUNTFLAG is reset to 0
        let mut systick = SYSTICK.borrow(cs).replace(None);
        let _ = systick.as_mut().unwrap().has_wrapped();
        SYSTICK.borrow(cs).set(systick);

        // Execute optional callback function
        let callback = CALLBACK_FN.borrow(cs).replace(None);
        if let Some(callback) = callback {
            callback(SYSTICK_COUNTER.load(Ordering::Relaxed));
        }
        CALLBACK_FN.borrow(cs).set(callback);
    });
}
