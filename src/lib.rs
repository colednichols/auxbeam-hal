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
                // Each switch gets its own entire byte (no bit-shifting needed)
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
                4 + p_len + 1 // Header + Payload + Checksum
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
