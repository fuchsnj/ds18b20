use embedded_hal::delay::DelayUs;

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum Resolution {
    Bits9 = 0b00011111,
    Bits10 = 0b00111111,
    Bits11 = 0b01011111,
    Bits12 = 0b01111111,
}

impl Resolution {
    pub fn max_measurement_time_millis(&self) -> u16 {
        match self {
            Resolution::Bits9 => 94,
            Resolution::Bits10 => 188,
            Resolution::Bits11 => 375,
            Resolution::Bits12 => 750,
        }
    }

    /// Blocks for the amount of time required to finished measuring temperature
    /// using this resolution
    pub fn delay_for_measurement_time(&self, delay: &mut impl DelayUs) {
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

    pub(crate) fn to_config_register(&self) -> u8 {
        *self as u8
    }
}
