pub mod display;
use anyhow::Result;
use esp_idf_hal::prelude::*;
use std::sync::mpsc;

use self::display::DisplayMessage;

pub fn init() -> Result<mpsc::SyncSender<DisplayMessage>> {
    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    #[cfg(feature = "ttgo")]
    let msg_sender = display::start(
        pins.gpio4,
        pins.gpio16,
        pins.gpio23,
        peripherals.spi2,
        pins.gpio18,
        pins.gpio19,
        pins.gpio5,
    )?;

    let msg_sender = display::start(
        pins.gpio4,
        pins.gpio16,
        pins.gpio13,
        peripherals.spi2,
        pins.gpio18,
        pins.gpio19,
        pins.gpio5,
    )?;

    msg_sender.send(DisplayMessage::Message("Display ready".to_string()))?;

    Ok(msg_sender)
}
