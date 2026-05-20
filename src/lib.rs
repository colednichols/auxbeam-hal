#![no_std]

use core::u8;

#[derive(Debug, Copy, Clone)]
pub struct SwitchMatrix {
    /// The number of physical switches on this specific panel (e.g., 4, 6, 8)
    pub count: u8,
    /// The resting or active state of each switch (Index 0 = Switch 1)
    pub states: [u8; 16],
}

impl Default for SwitchMatrix {
    fn default() -> Self {
        Self {
            count: 8,           // Default to 8-gang if not specified
            states: [0x08; 16], // 0x08 is the "Ignore Mask" default
        }
    }
}

impl SwitchMatrix {
    // Return matrix and length in tuple
    pub fn to_bytes(&self) -> ([u8; 8], usize) {
        let mut payload = [0x00; 8];
        let payload_len = (self.count as usize + 1) / 2;

        for i in 0..payload_len {
            let high_switch = self.states.get(i * 2).copied().unwrap_or(0x08);
            let low_switch = self.states.get((i * 2) + 1).copied().unwrap_or(0x08);

            // Shift the first switch to the high nibble, mask the second to the low nibble
            payload[i] = (high_switch << 4) | (low_switch & 0x0F);
        }

        (payload, payload_len)
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

#[derive(Debug, Copy, Clone)]
pub enum Command {
    Switch(SwitchMatrix),            // 0x08
    MasterSwitch(SwitchMatrix),      // 0x07
    Backlight(PanelBacklightMatrix), // 0x0C
    Group(GroupMatrix),              // 0x02
}
