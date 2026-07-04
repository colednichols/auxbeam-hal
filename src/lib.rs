#![no_std]

use core::u8;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum SwitchState {
    ToggleOff = 0x00,
    ToggleOn = 0x01,
    MomentOff = 0x02,
    MomentOn = 0x03,
    StrobeOff = 0x04,
    StrobeOn = 0x05,
    UnusedOff = 0x06,
    UnusedOn = 0x07,
    Ignore = 0x08,
}
impl SwitchState {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0x00 => SwitchState::ToggleOff,
            0x01 => SwitchState::ToggleOn,
            0x02 => SwitchState::MomentOff,
            0x03 => SwitchState::MomentOn,
            0x04 => SwitchState::StrobeOff,
            0x05 => SwitchState::StrobeOn,
            0x06 => SwitchState::UnusedOff,
            0x07 => SwitchState::UnusedOn,
            _ => SwitchState::Ignore,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SwitchMatrix {
    /// The number of physical switches on this specific panel (e.g., 4, 6, 8)
    pub count: u8,
    /// The resting or active state of each switch (Index 0 = Switch 1)
    pub states: [SwitchState; 16],
}

impl Default for SwitchMatrix {
    fn default() -> Self {
        Self {
            count: 8, // Default to 8-gang if not specified
            states: [SwitchState::Ignore; 16],
        }
    }
}

impl SwitchMatrix {
    // Return matrix and length in tuple
    pub fn to_bytes(&self) -> ([u8; 8], usize) {
        let mut payload = [0x00; 8];
        let payload_len = (self.count as usize + 1) / 2;

        for i in 0..payload_len {
            let high_switch = self
                .states
                .get(i * 2)
                .copied()
                .unwrap_or(SwitchState::Ignore) as u8;
            let low_switch = self
                .states
                .get((i * 2) + 1)
                .copied()
                .unwrap_or(SwitchState::Ignore) as u8;

            // Shift the first switch to the high nibble, mask the second to the low nibble
            payload[i] = (high_switch << 4) | (low_switch & 0x0F);
        }

        (payload, payload_len)
    }

    pub fn set_multiple(&mut self, switches: &[usize], state: SwitchState) {
        for &switch in switches {
            // Ignore out-of-bounds input
            if switch > 0 && switch <= self.count as usize {
                self.states[switch - 1] = state;
            }
        }
    }

    /// Convenience wrapper to latch multiple switches ON.
    pub fn turn_on(&mut self, switches: &[usize]) {
        self.set_multiple(switches, SwitchState::ToggleOn);
    }

    /// Convenience wrapper to latch multiple switches OFF.
    pub fn turn_off(&mut self, switches: &[usize]) {
        self.set_multiple(switches, SwitchState::ToggleOff);
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct PanelBacklightMatrix {
    pub brightness: u8,
    pub red: u8,
    pub blue: u8,
    pub green: u8,
}

impl PanelBacklightMatrix {
    pub fn to_bytes(&self) -> [u8; 5] {
        [
            self.brightness,
            self.red,
            self.blue,
            self.green,
            0x00, // Padding: Maybe white? Needs further testing
        ]
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct GroupMatrix {
    pub active: bool,
    pub group_id: u8,
    pub count: u8,
    pub switches: [u8; 16],
}

impl GroupMatrix {
    pub fn to_bytes(&self) -> ([u8; 19], usize) {
        let mut payload = [0x00; 19];

        if self.active {
            payload[0] = 0x01; // Action Flag
            payload[1] = self.group_id; // Target Group
            payload[2] = self.count; // Switch Count (N)

            // Clamp the count to prevent buffer overflows if user inputs > 16
            let safe_n = (self.count as usize).min(16);

            for i in 0..safe_n {
                // Each switch gets its own entire byte
                payload[3 + i] = self.switches[i];
            }

            // Length is 3 header bytes + N switch bytes
            (payload, 3 + safe_n)
        } else {
            // Action: Delete / Clear
            payload[0] = 0x00; // Action Flag
            payload[1] = self.group_id; // Target Group

            (payload, 2)
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameDestination {
    Box = 0x00,
    Panel = 0xFF,
}

#[derive(Debug, Copy, Clone)]
pub enum Command {
    Switch(SwitchMatrix),             // 0x08 / 0x18
    MasterSwitch(bool, SwitchMatrix), // 0x07
    Backlight(PanelBacklightMatrix),  // 0x0C
    Group(GroupMatrix),               // 0x02
    Strobe(u8),                       // 0x0B / 0x1B
    BootSignal([u8; 5]),              // 0x09
}
impl Command {
    // Serialize command into wire-ready frame
    // Returns frame as buffer and valid length
    pub fn as_frame(&self, sequence_id: u8) -> ([u8; 24], usize) {
        let mut frame = [0x00; 24];
        frame[0] = sequence_id;

        let len = match self {
            Command::Switch(matrix) => {
                frame[1] = FrameDestination::Box as u8;
                frame[2] = 0x08; // Command ID
                frame[3] = matrix.count;

                let (payload, p_len) = matrix.to_bytes();
                frame[4..4 + p_len].copy_from_slice(&payload[..p_len]);
                4 + p_len + 1 // Header + Payload + Checksum
            }
            Command::MasterSwitch(master_state, matrix) => {
                frame[1] = FrameDestination::Box as u8;
                frame[2] = 0x07;
                frame[3] = if *master_state { 0x00 } else { 0x01 };

                let (payload, p_len) = matrix.to_bytes();
                frame[4..4 + p_len].copy_from_slice(&payload[..p_len]);
                4 + p_len + 1 // Header + Payload + Checksum
            }
            Command::Backlight(backlight_param) => {
                frame[1] = FrameDestination::Panel as u8;
                frame[2] = 0x0C;

                let payload = backlight_param.to_bytes();
                frame[3..3 + 5].copy_from_slice(&payload[..5]);
                3 + 5 + 1
            }
            Command::Group(matrix) => {
                frame[1] = FrameDestination::Panel as u8;
                frame[2] = 0x02;

                let (payload, p_len) = matrix.to_bytes();
                frame[3..3 + p_len].copy_from_slice(&payload[..p_len]);
                3 + p_len + 1 // Header + Payload + Checksum
            }
            Command::Strobe(strobe_length) => {
                frame[1] = FrameDestination::Box as u8;
                frame[2] = 0x0B;
                frame[3] = *strobe_length;

                5
            }
            Command::BootSignal(payload) => {
                frame[1] = 0xFF;
                frame[2] = 0x09;

                frame[3..8].copy_from_slice(payload);
                9
            }
        };

        // Modulo 256 Checksum Calculator
        let mut checksum: u16 = 0;
        for i in 0..(len - 1) {
            checksum = checksum.wrapping_add(frame[i] as u16);
        }
        frame[len - 1] = (checksum & 0xFF) as u8;

        (frame, len)
    }
}

pub struct AuxbeamParser {
    buffer: [u8; 24],
    index: usize,
}

impl Default for AuxbeamParser {
    fn default() -> Self {
        Self {
            buffer: [0x00; 24],
            index: 0,
        }
    }
}

impl AuxbeamParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a raw byte into the sliding window.
    /// Returns Some((SequenceID, Command)) when a valid frame completes.
    pub fn feed(&mut self, byte: u8) -> Option<(u8, Command)> {
        // 1. Add byte to buffer
        if self.index < 24 {
            self.buffer[self.index] = byte;
            self.index += 1;
        } else {
            self.shift_window();
            self.buffer[23] = byte;
        }

        // 2. Check if we have enough bytes to process, unwrap the length
        let expected_len = match self.get_expected_length() {
            None => return None, // Keep waiting for more bytes, do not shift.
            Some(0) => {
                // Invalid command ID detected. Shift to dump the bad header.
                self.shift_window();
                return None;
            }
            Some(len) => len, // Valid length extracted, proceed to step 3.
        };

        // 3. If we hit the expected length, validate and decode
        // (expected_len is now a standard usize, so this comparison works)
        if self.index == expected_len {
            if self.calculate_checksum(expected_len - 1) == self.buffer[expected_len - 1] {
                let event = self.decode_frame();
                self.index = 0; // Reset for the next frame
                return event;
            } else {
                // Checksum failed. Shift window by 1 to hunt for the real frame.
                self.shift_window();
            }
        } else if self.index > expected_len {
            self.shift_window();
        }

        None
    }

    /// Calculates the dynamic length based on the Command ID and frame headers.
    /// Returns 0 if the Command ID is unknown/invalid.
    fn get_expected_length(&self) -> Option<usize> {
        if self.index < 3 {
            return None;
        } // None = Keep waiting

        match self.buffer[2] {
            0x08 | 0x18 => {
                if self.index < 4 {
                    return None;
                }
                let payload_len = (self.buffer[3] as usize + 1) / 2;
                Some(4 + payload_len + 1)
            }
            0x0B | 0x1B => Some(5),
            0x0C | 0x07 | 0x09 => Some(9),
            0x17 => Some(10),
            0x02 => {
                if self.index < 4 {
                    return None;
                }
                if self.buffer[3] == 0x00 {
                    return Some(6);
                } // Delete
                if self.buffer[3] == 0x01 {
                    if self.index < 6 {
                        return None;
                    }
                    return Some(7 + self.buffer[5] as usize); // Create
                }
                Some(0) // Invalid Action Flag
            }
            _ => Some(0), // Invalid Command
        }
    }

    /// Unpacks a validated byte buffer back into the Command enum
    fn decode_frame(&self) -> Option<(u8, Command)> {
        let seq = self.buffer[0];
        let cmd = self.buffer[2];

        let command = match cmd {
            0x08 | 0x18 => {
                let count = self.buffer[3];
                let mut matrix = SwitchMatrix {
                    count,
                    states: [SwitchState::Ignore; 16],
                };
                let payload_len = (count as usize + 1) / 2;

                for i in 0..payload_len {
                    let byte = self.buffer[4 + i];
                    if i * 2 < 16 {
                        matrix.states[i * 2] = SwitchState::from_u8(byte >> 4);
                    }
                    if i * 2 + 1 < 16 {
                        matrix.states[i * 2 + 1] = SwitchState::from_u8(byte & 0x0F);
                    }
                }
                Command::Switch(matrix)
            }
            0x0C => Command::Backlight(PanelBacklightMatrix {
                brightness: self.buffer[3],
                red: self.buffer[4],
                blue: self.buffer[5],
                green: self.buffer[6],
            }),
            0x07 | 0x17 => {
                // 0x07 puts the target state at index 3.
                // 0x17 puts the current state at 3, and the set state at 4.
                let state_byte = if cmd == 0x17 {
                    self.buffer[4]
                } else {
                    self.buffer[3]
                };
                let active = state_byte == 0x00;
                Command::MasterSwitch(active, SwitchMatrix::default())
            }
            0x02 => {
                let active = self.buffer[3] == 0x01;
                let group_id = self.buffer[4];
                let count = if active { self.buffer[5] } else { 0 };
                let mut switches = [0x00; 16];

                if active {
                    let safe_n = (count as usize).min(16);
                    for i in 0..safe_n {
                        switches[i] = self.buffer[6 + i];
                    }
                }
                Command::Group(GroupMatrix {
                    active,
                    group_id,
                    count,
                    switches,
                })
            }
            0x0B | 0x1B => Command::Strobe(self.buffer[3]),
            0x09 => {
                let mut payload = [0x00; 5];
                payload.copy_from_slice(&self.buffer[3..8]);
                Command::BootSignal(payload)
            }
            _ => return None,
        };

        Some((seq, command))
    }

    fn calculate_checksum(&self, len: usize) -> u8 {
        let mut sum: u16 = 0;
        for i in 0..len {
            sum = sum.wrapping_add(self.buffer[i] as u16);
        }
        (sum & 0xFF) as u8
    }

    fn shift_window(&mut self) {
        for i in 1..24 {
            self.buffer[i - 1] = self.buffer[i];
        }
        self.index = self.index.saturating_sub(1);
    }
}
