#![doc = include_str!("../README.md")]
#![no_std]
#![allow(dead_code)]

pub mod delay;

use cortex_m::interrupt;

#[cfg(feature = "irq_handler")]
use cortex_m_rt::exception;

/// SysTick peripheral.
static mut SYSTICK: Option<cortex_m::peripheral::SYST> = None;

/// SysTick counter increased in interrupt.
static mut SYSTICK_COUNTER: u64 = 0;

/// System clock frequency in MHz.
static mut CLOCK_FREQ_MHZ: u32 = 0;

/// SysTick frequency in Hz.
static mut TICK_FREQ: u32 = 0;

/// Optional callback function triggered within SysTick interrupt
static mut CALLBACK_FN: Option<fn(u64)> = None;

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

    unsafe {
        // The tick counter should start with 0 after init
        SYSTICK_COUNTER = 0;

        // These values need to be stored for further calculations
        CLOCK_FREQ_MHZ = clock_freq / 1000000;
        TICK_FREQ = tick_freq;
    }

    // Setup the timer registers with the required values
    let reload = (clock_freq / tick_freq) - 1;
    syst.set_reload(reload);
    syst.clear_current();

    // Finally start the interrupt and let everything run
    syst.enable_interrupt();

    unsafe { SYSTICK = Some(syst) }
}

/// Returns the SysTick timer.
///
/// Use this function to get back ownership of the peripheral.
/// No prior actions like `stop()` are performed by this function.
pub fn free() -> cortex_m::peripheral::SYST {
    unsafe { SYSTICK.take().unwrap() }
}

/// Starts the counter.
///
/// Initialisation must be done before calling this function.
/// Use `stop()` to halt the counter again.
pub fn start() {
    unsafe { SYSTICK.as_mut().unwrap().enable_counter() }
}

/// Stops the counter.
pub fn stop() {
    unsafe { SYSTICK.as_mut().unwrap().disable_counter() }
}

/// Resets the counter.
pub fn reset() {
    interrupt::free(|_| unsafe {
        SYSTICK.as_mut().unwrap().clear_current();
        SYSTICK_COUNTER = 0
    });
}

/// Returns the tick count.
pub fn ticks() -> u64 {
    interrupt::free(|_| unsafe { SYSTICK_COUNTER })
}

/// Returns the number of core clock cycles.
pub fn clock_cycles() -> u64 {
    interrupt::free(|_| {
        let mut ticks = unsafe { SYSTICK_COUNTER };
        let syst = unsafe { SYSTICK.as_mut().unwrap() };
        let load = syst.rvr.read();
        let val = syst.cvr.read();

        if syst.has_wrapped() {
            // This catches the case when the counter has reached 0 after
            // the last interrupt but before reading the value in the
            // statement above.
            ticks += 1;
        }

        ((load as u64 + 1) * ticks) + (load - val) as u64
    })
}

/// Returns elapsed milliseconds.
pub fn millis() -> u64 {
    unsafe { ticks() * 1000 / TICK_FREQ as u64 }
}

/// Returns elapsed microseconds.
pub fn micros() -> u64 {
    unsafe { clock_cycles() / CLOCK_FREQ_MHZ as u64 }
}

/// Set an interrupt callback function.
///
/// The provided callback function is called on each SysTick interrupt
/// after updating the tick count and passed its value as argument
pub fn set_callback(callback: fn(u64)) {
    unsafe {
        CALLBACK_FN = Some(callback);
    };
}

/// Clear the interrupt callback function.
pub fn clear_callback() {
    unsafe {
        CALLBACK_FN = None;
    };
}

/// External interrupt call.
///
/// This function must be called from the external SysTick handler
/// when the `irq_handler` feature is disabled.
#[cfg(not(feature = "irq_handler"))]
pub fn interrupt() {
    irq();
}

/// Called on SysTick interrupt, either internally or via the `interrupt()` function.
fn irq() {
    unsafe {
        // Increase the counter
        SYSTICK_COUNTER += 1;

        // Read the status register to ensure COUNTFLAG is reset to 0
        let _ = SYSTICK.as_mut().unwrap().has_wrapped();

        // Execute optional callback function
        if let Some(callback) = CALLBACK_FN {
            callback(SYSTICK_COUNTER);
        }
    }
}

/// SysTick interrupt handler
#[cfg(feature = "irq_handler")]
#[exception]
#[allow(non_snake_case)]
fn SysTick() {
    irq();
}
