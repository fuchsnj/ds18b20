#![no_std]
#![doc = include_str!("../README.md")]

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use one_wire_bus::{self, Address, OneWire, OneWireError, OneWireResult};

/// The 1-Wire family code for the DS18B20.
pub const FAMILY_CODE: u8 = 0x28;

pub mod commands;
mod resolution;

use one_wire_bus::crc::check_crc8;
pub use resolution::Resolution;

/// All of the data that can be read from the sensor.
#[derive(Debug)]
pub struct SensorData {
    /// Temperature in degrees Celsius. Defaults to 85 on startup
    pub temperature: f32,

    /// The current resolution configuration
    pub resolution: Resolution,

    /// If the last recorded temperature is at or below this, the sensor is put in an alarm state
    pub alarm_temp_low: i8,

    /// If the last recorded temperature is at or above this, the sensor is put in an alarm state
    pub alarm_temp_high: i8,
}

/// A DS18B20 sensor identified by its 1-Wire ROM address.
pub struct Ds18b20 {
    address: Address,
}

impl Ds18b20 {
    /// Creates a sensor handle after validating its address family code.
    pub fn new<E>(address: Address) -> OneWireResult<Ds18b20, E> {
        if address.family_code() == FAMILY_CODE {
            Ok(Ds18b20 { address })
        } else {
            Err(OneWireError::FamilyCodeMismatch)
        }
    }

    /// Returns the device address.
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Starts a temperature conversion for this sensor.
    ///
    /// Wait for the configured resolution’s conversion time before reading the result.
    pub fn start_temp_measurement<T, E>(
        &self,
        onewire: &mut OneWire<T>,
        delay: &mut impl DelayNs,
    ) -> OneWireResult<(), E>
    where
        T: InputPin + OutputPin<Error = E>,
    {
        onewire.send_command(commands::CONVERT_TEMP, Some(&self.address), delay)
    }

    /// Reads the nine raw scratchpad bytes and checks their CRC.
    pub fn read_scratchpad<T, E>(
        &self,
        onewire: &mut OneWire<T>,
        delay: &mut impl DelayNs,
    ) -> OneWireResult<[u8; 9], E>
    where
        T: InputPin + OutputPin<Error = E>,
    {
        read_scratchpad(&self.address, onewire, delay)
    }

    /// Reads the last converted temperature, resolution, and alarm thresholds.
    pub fn read_data<T, E>(
        &self,
        onewire: &mut OneWire<T>,
        delay: &mut impl DelayNs,
    ) -> OneWireResult<SensorData, E>
    where
        T: InputPin + OutputPin<Error = E>,
    {
        decode_scratchpad(self.read_scratchpad(onewire, delay)?)
    }

    /// Writes alarm thresholds and resolution to the scratchpad.
    ///
    /// Call [Self::save_to_eeprom] to persist these settings across power cycles.
    pub fn set_config<T, E>(
        &self,
        alarm_temp_low: i8,
        alarm_temp_high: i8,
        resolution: Resolution,
        onewire: &mut OneWire<T>,
        delay: &mut impl DelayNs,
    ) -> OneWireResult<(), E>
    where
        T: InputPin + OutputPin<Error = E>,
    {
        onewire.send_command(commands::WRITE_SCRATCHPAD, Some(&self.address), delay)?;
        onewire.write_bytes(
            &[
                alarm_temp_high as u8,
                alarm_temp_low as u8,
                resolution.to_config_register(),
            ],
            delay,
        )
    }

    /// Copies the scratchpad configuration to EEPROM and waits for the write.
    pub fn save_to_eeprom<T, E>(
        &self,
        onewire: &mut OneWire<T>,
        delay: &mut impl DelayNs,
    ) -> OneWireResult<(), E>
    where
        T: InputPin + OutputPin<Error = E>,
    {
        save_to_eeprom(Some(&self.address), onewire, delay)
    }

    /// Recalls the EEPROM configuration into the scratchpad and waits for completion.
    pub fn recall_from_eeprom<T, E>(
        &self,
        onewire: &mut OneWire<T>,
        delay: &mut impl DelayNs,
    ) -> OneWireResult<(), E>
    where
        T: InputPin + OutputPin<Error = E>,
    {
        recall_from_eeprom(Some(&self.address), onewire, delay)
    }
}

/// Broadcasts a temperature conversion command to all devices on the 1-Wire bus.
pub fn start_simultaneous_temp_measurement<T, E>(
    onewire: &mut OneWire<T>,
    delay: &mut impl DelayNs,
) -> OneWireResult<(), E>
where
    T: InputPin + OutputPin<Error = E>,
{
    onewire.send_command(commands::CONVERT_TEMP, None, delay)
}

/// Recalls the EEPROM configuration into the scratchpad on all devices.
pub fn simultaneous_recall_from_eeprom<T, E>(
    onewire: &mut OneWire<T>,
    delay: &mut impl DelayNs,
) -> OneWireResult<(), E>
where
    T: InputPin + OutputPin<Error = E>,
{
    recall_from_eeprom(None, onewire, delay)
}

/// Copies the scratchpad configuration to EEPROM on all devices.
pub fn simultaneous_save_to_eeprom<T, E>(
    onewire: &mut OneWire<T>,
    delay: &mut impl DelayNs,
) -> OneWireResult<(), E>
where
    T: InputPin + OutputPin<Error = E>,
{
    save_to_eeprom(None, onewire, delay)
}

/// Reads the nine raw scratchpad bytes from a device and checks their CRC.
pub fn read_scratchpad<T, E>(
    address: &Address,
    onewire: &mut OneWire<T>,
    delay: &mut impl DelayNs,
) -> OneWireResult<[u8; 9], E>
where
    T: InputPin + OutputPin<Error = E>,
{
    onewire.send_command(commands::READ_SCRATCHPAD, Some(address), delay)?;
    let mut scratchpad = [0; 9];
    onewire.read_bytes(&mut scratchpad, delay)?;
    check_crc8(&scratchpad)?;
    Ok(scratchpad)
}

fn decode_scratchpad<E>(scratchpad: [u8; 9]) -> OneWireResult<SensorData, E> {
    let resolution =
        Resolution::from_config_register(scratchpad[4]).ok_or(OneWireError::CrcMismatch)?;
    // The DS18B20 keeps the same 1/16 °C scale at every resolution.
    let undefined_bits = match resolution {
        Resolution::Bits9 => 0b111,
        Resolution::Bits10 => 0b011,
        Resolution::Bits11 => 0b001,
        Resolution::Bits12 => 0,
    };
    let raw_temp = i16::from_le_bytes([scratchpad[0], scratchpad[1]]);
    let temperature = f32::from(raw_temp & !undefined_bits) / 16.0;
    Ok(SensorData {
        temperature,
        resolution,
        alarm_temp_high: i8::from_le_bytes([scratchpad[2]]),
        alarm_temp_low: i8::from_le_bytes([scratchpad[3]]),
    })
}

fn recall_from_eeprom<T, E>(
    address: Option<&Address>,
    onewire: &mut OneWire<T>,
    delay: &mut impl DelayNs,
) -> OneWireResult<(), E>
where
    T: InputPin + OutputPin<Error = E>,
{
    onewire.send_command(commands::RECALL_EEPROM, address, delay)?;

    // wait for the recall to finish (up to 10ms)
    let max_retries = (10000 / one_wire_bus::READ_SLOT_DURATION_MICROS) + 1;
    for _ in 0..max_retries {
        if onewire.read_bit(delay)? {
            return Ok(());
        }
    }
    Err(OneWireError::Timeout)
}

fn save_to_eeprom<T, E>(
    address: Option<&Address>,
    onewire: &mut OneWire<T>,
    delay: &mut impl DelayNs,
) -> OneWireResult<(), E>
where
    T: InputPin + OutputPin<Error = E>,
{
    onewire.send_command(commands::COPY_SCRATCHPAD, address, delay)?;
    delay.delay_ms(10);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_signed_temperatures_at_all_resolutions() {
        let cases = [
            (Resolution::Bits9, 0x0197, 25.0),
            (Resolution::Bits9, 0xfe6f, -25.5),
            (Resolution::Bits10, 0x0197, 25.25),
            (Resolution::Bits10, 0xfe6f, -25.25),
            (Resolution::Bits11, 0x0197, 25.375),
            (Resolution::Bits11, 0xfe6f, -25.125),
            (Resolution::Bits12, 0x0197, 25.4375),
            (Resolution::Bits12, 0xfe6f, -25.0625),
        ];

        for (resolution, raw, expected) in cases {
            let mut scratchpad = [0; 9];
            scratchpad[..2].copy_from_slice(&u16::to_le_bytes(raw));
            scratchpad[4] = resolution.to_config_register();

            let data = decode_scratchpad::<core::convert::Infallible>(scratchpad).unwrap();
            assert_eq!(data.temperature, expected, "{:?}", resolution);
        }
    }
}
