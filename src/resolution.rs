use embedded_hal::delay::DelayNs;

/// The configured temperature measurement resolution.
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum Resolution {
    /// 9-bit measurements in 0.5 °C increments.
    Bits9 = 0b00011111,
    /// 10-bit measurements in 0.25 °C increments.
    Bits10 = 0b00111111,
    /// 11-bit measurements in 0.125 °C increments.
    Bits11 = 0b01011111,
    /// 12-bit measurements in 0.0625 °C increments.
    Bits12 = 0b01111111,
}

impl Resolution {
    /// Returns the maximum conversion time, rounded up to whole milliseconds.
    pub fn max_measurement_time_millis(&self) -> u16 {
        match self {
            Resolution::Bits9 => 94,
            Resolution::Bits10 => 188,
            Resolution::Bits11 => 375,
            Resolution::Bits12 => 750,
        }
    }

    /// Waits long enough for a temperature conversion at this resolution.
    pub fn delay_for_measurement_time(&self, delay: &mut impl DelayNs) {
        delay.delay_ms(self.max_measurement_time_millis().into());
    }

    pub(crate) fn from_config_register(config: u8) -> Option<Resolution> {
        match config {
            0b00011111 => Some(Resolution::Bits9),
            0b00111111 => Some(Resolution::Bits10),
            0b01011111 => Some(Resolution::Bits11),
            0b01111111 => Some(Resolution::Bits12),
            _ => None,
        }
    }

    pub(crate) fn to_config_register(self) -> u8 {
        self as u8
    }
}
