# auxbeam-hal
A robust, `#![no_std]` compatible Hardware Abstraction Layer (HAL) for interacting with Auxbeam switch panels and solid-state relay boxes.

This library implements the reverse-engineered UART protocol used between the switch panel and relay box, providing a memory-safe, zero-allocation sliding window parser and a strongly typed command interface. It is ideal for custom vehicle integrations, allowing microcontrollers like the ESP32 to directly monitor panel button presses or control the relay box independent of the factory panel.

## Features
* no_std architecture: Highly adaptable to any microcontroller/processor.
* Resilient Parsing: The AuxbeamParser utilizes a sliding window approach to gracefully handle dropped bytes, electrical noise, and stream desynchronization on automotive UART buses.
* Strongly Typed Commands: Easily construct switch matrix states, grouped actions, or strobe timings, and serialize them into checksum-validated wire frames.
* Closed-Loop Capable: Decode hardware acknowledgments to verify physical relay states and detect manual overrides from the physical panel.

## Hardware
This protocol was reverse-engineered and tested against the following hardware:
* Tested: Auxbeam 8-Gang Switch Panel (Standard RGB model)
  Connector on relay box: GND, 3.3 V, RX, TX (Panel/controller RX connects to relay box TX and vise versa)
*Note: The protocol structure suggests it will likely work with other models, but this is unverified. I have done my best to make this scalable to other models.*

## Installation
Add the library to your Cargo.toml:
```toml
[dependencies]
auxbeam_hal = "0.1.0"
```
## Usage Example: Closed-Loop Controller
```rust
use auxbeam_hal::{AuxbeamParser, Command, SwitchMatrix, SwitchState};
// Note: Requires an underlying hardware UART driver for your specific MCU

fn main() {
    let mut parser = AuxbeamParser::new();
    let mut sequence_id: u8 = 0;
    
    // 1. Configure the desired state
    let mut matrix = SwitchMatrix::default();
    matrix.set_multiple(&[1], SwitchState::ToggleOn); // Target physical Switch 1
    
    // 2. Serialize the command to raw bytes
    sequence_id = sequence_id.wrapping_add(1);
    let (frame, len) = Command::Switch(matrix).as_frame(sequence_id);
    
    // 3. Send over UART (implementation depends on your HAL, e.g., esp-idf-hal)
    // uart.write(&frame[..len]).unwrap();
    
    // 4. Parse incoming UART bytes to confirm execution
    let mut rx_buf = [0u8; 16];
    // if let Ok(bytes_read) = uart.read(&mut rx_buf, 50) {
    //     for i in 0..bytes_read {
    //         // feed() returns Some() when a valid, checksum-verified frame completes
    //         if let Some((_, Command::Switch(rx_matrix))) = parser.feed(rx_buf[i]) {
    //             let actual_state = rx_matrix.states[0];
    //             println!("Confirmed hardware state for Switch 1: {:?}", actual_state);
    //         }
    //     }
    // }
}
```
