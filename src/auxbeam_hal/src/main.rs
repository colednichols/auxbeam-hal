#![no_std]

use core::u8;

#[derive(Debug, Copy, Clone, Default)]
pub struct SwitchMatrix {
    pub s1: u8,
    pub s2: u8,
    pub s3: u8,
    pub s4: u8,
    pub s5: u8,
    pub s6: u8,
    pub s7: u8,
    pub s8: u8,
}

impl SwitchMatrix {
    // to_bytes: construct 5 byte matrix payload
    pub fn to_bytes(&self) -> [u8; 5] {
        [
            0x08, // Potentially address, allowing for 16 gang panels
            (self.s1 & 0xF0) | (self.s2 & 0x0F),
            (self.s3 & 0xF0) | (self.s4 & 0x0F),
            (self.s5 & 0xF0) | (self.s6 & 0x0F),
            (self.s7 & 0xF0) | (self.s8 & 0x0F),
        ]
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct MasterSwitchMatrix {
    pub master_state: bool,
    pub switch_matrix: SwitchMatrix,
}

impl MasterSwitchMatrix {
    fn to_bytes(&self) -> [u8; 5] {
        let a = self.master_state as u8;
        let [_, b, c, d, e] = self.switch_matrix.to_bytes();
        [a, b, c, d, e]
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
    active: bool,
}

#[derive(Debug, Copy, Clone)]
pub enum Command {
    Switch(SwitchMatrix),             // 0x08
    MasterSwitch(MasterSwitchMatrix), // 0x07
    Backlight(PanelBacklightMatrix),  // 0x0C
    Group(GroupMatrix),               // 0x02
}
