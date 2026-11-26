# DS18B20 temperature sensor

[![CI](https://github.com/fuchsnj/ds18b20/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/fuchsnj/ds18b20/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/ds18b20.svg)](https://crates.io/crates/ds18b20)
[![API](https://docs.rs/ds18b20/badge.svg)](https://docs.rs/ds18b20)

A `no_std` driver for the [DS18B20 temperature sensor](https://www.analog.com/media/en/technical-documentation/data-sheets/ds18b20.pdf)
using [embedded-hal 1.0](https://github.com/rust-embedded/embedded-hal).

## Bus setup

Use your board's HAL to configure an open-drain GPIO that implements both
`embedded_hal::digital::InputPin` and `OutputPin`, and supply a delay that
implements `embedded_hal::delay::DelayNs`. Connect DQ to the GPIO with a pull-up
resistor (typically 4.7 kΩ). The examples assume externally powered sensors.

Use [one-wire-bus 1.0](https://crates.io/crates/one-wire-bus) to create the
`OneWire` bus passed to the driver. When upgrading from `ds18b20` 0.1, update
both bus and HAL dependencies and replace `DelayUs` and `DelayMs` with `DelayNs`.

```toml
[dependencies]
embedded-hal = "1.0.0"
one-wire-bus = "1.0.0"
```

Add this checkout of `ds18b20` as a path dependency when trying the examples locally.

## Read temperatures

Pass the configured pin and delay from your board setup to `read_temperatures`.
The callback receives each sensor's ROM address and reading, and can log the
results or pass them to the rest of the application.

```rust,no_run
use ds18b20::{Ds18b20, Resolution, FAMILY_CODE};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use one_wire_bus::{Address, OneWire, OneWireResult};

fn read_temperatures<P: InputPin + OutputPin>(
    pin: P,
    delay: &mut impl DelayNs,
    mut report: impl FnMut(Address, f32),
) -> OneWireResult<(), P::Error> {
    let mut bus = OneWire::new(pin)?;
    ds18b20::start_simultaneous_temp_measurement(&mut bus, delay)?;
    Resolution::Bits12.delay_for_measurement_time(delay);

    let mut search_state = None;
    while let Some((address, state)) = bus.device_search(search_state.as_ref(), false, delay)? {
        search_state = Some(state);
        if address.family_code() == FAMILY_CODE {
            let sensor = Ds18b20::new(address)?;
            let data = sensor.read_data(&mut bus, delay)?;
            report(address, data.temperature);
        }
    }
    Ok(())
}
```

## Update configuration

Use the sensor's ROM address from a bus search, and pass the initialized
`OneWire` bus and board delay to this function. The example keeps the current
resolution and saves the new alarm thresholds to EEPROM.

```rust,no_run
use ds18b20::{Ds18b20, SensorData};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use one_wire_bus::{Address, OneWire, OneWireResult};

fn update_alarms<P: InputPin + OutputPin>(
    address: Address,
    bus: &mut OneWire<P>,
    delay: &mut impl DelayNs,
) -> OneWireResult<SensorData, P::Error> {
    let sensor = Ds18b20::new(address)?;
    let original = sensor.read_data(bus, delay)?;
    sensor.set_config(18, 24, original.resolution, bus, delay)?;
    sensor.save_to_eeprom(bus, delay)?;

    sensor.recall_from_eeprom(bus, delay)?;
    sensor.read_data(bus, delay)
}
```
