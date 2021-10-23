# cortex-m-systick

This Rust crate initializes the Cortex-M SysTick timer with a specific tick frequency and provides basic functions for time-based calculations.

It sets up the SysTick interrupt to increase an 32-bit tick counter. Consequently, a typical 1000Hz tick frequency will result in a period of nearly 50 days before overflowing.

## Usage Examples

```rust
// Import the crate
use cortex_m_systick as systick;

// Configure SysTick for 1000Hz interval on 80MHz core clock
let cp = cortex_m::Peripherals::take().unwrap();
systick::init_freq(cp.SYST, 80000000, 1000);
systick::start();

// Get number of milliseconds from start
let ms = systick::millis();

// Get number of microseconds from start
let us = systick::micros();

// Delay 20 milliseconds
systick::delay::delay_ms(20);

// Delay 50 microseconds
systick::delay::delay_us(50);
```

## License

Published under the MIT license.

Author: Oliver Rockstedt <info@sourcebox.de>