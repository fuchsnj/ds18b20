//! DS18B20 function command bytes.

/// Starts a temperature conversion.
pub const CONVERT_TEMP: u8 = 0x44;
/// Writes the alarm thresholds and configuration register into the scratchpad.
pub const WRITE_SCRATCHPAD: u8 = 0x4E;
/// Reads the scratchpad.
pub const READ_SCRATCHPAD: u8 = 0xBE;
/// Copies the scratchpad configuration into EEPROM.
pub const COPY_SCRATCHPAD: u8 = 0x48;
/// Recalls the EEPROM configuration into the scratchpad.
pub const RECALL_EEPROM: u8 = 0xB8;
