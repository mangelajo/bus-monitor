use anyhow::Result;
use embedded_graphics::image::ImageRaw;
use embedded_graphics::mono_font::iso_8859_1::FONT_9X18_BOLD;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::prelude::*;
use embedded_graphics::text::*;

use esp_idf_hal::prelude::*;
use esp_idf_hal::{delay, gpio, spi};

#[cfg(feature = "ttgo")]
use st7789;

//#[cfg(esp32s3)]
use epd_waveshare::{color::*, epd3in7::*, prelude::*};

use std;
use std::sync::mpsc;

use crate::emtmadrid::ArrivalTime;

#[cfg(feature = "ttgo")]
pub fn start(
    backlight: gpio::Gpio4<gpio::Unknown>,
    dc: gpio::Gpio16<gpio::Unknown>,
    rst: gpio::Gpio23<gpio::Unknown>, // TTGO ESP32
    spi: spi::SPI2,
    sclk: gpio::Gpio18<gpio::Unknown>,
    sdo: gpio::Gpio19<gpio::Unknown>,
    cs: gpio::Gpio5<gpio::Unknown>,
) -> Result<mpsc::SyncSender<String>> {
    let config = <spi::config::Config as Default>::default().baudrate(26.MHz().into());

    let di = SPIInterfaceNoCS::new(
        spi::Master::<spi::SPI2, _, _, _, _>::new(
            spi,
            spi::Pins {
                sclk,
                sdo,
                sdi: Option::<gpio::Gpio21<gpio::Unknown>>::None,
                cs: Some(cs),
            },
            config,
        )?,
        dc.into_output()?,
    );

    let mut display = st7789::ST7789::new(di, rst.into_output()?, 240, 320);

    display
        .init(&mut delay::Ets)
        .map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;
    display
        .set_orientation(st7789::Orientation::Landscape)
        .map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;

    let top_left = Point::new(40, 53);

    display.clear(Rgb565::BLACK.into()).unwrap();

    let mut backlight = backlight.into_output()?;
    backlight.set_high()?;

    let (tx, rx) = mpsc::sync_channel::<String>(5);

    let _ = std::thread::Builder::new()
        .stack_size(16_000)
        .spawn(move || {
            let style = MonoTextStyle::new(&FONT_6X12, Rgb565::WHITE.into());
            let height = 12;
            let mut y = height;
            display.clear(Rgb565::BLACK.into()).unwrap();

            for msg in rx {
                let display = &mut display.translated(top_left);
                println!("Display: {}", msg);

                if msg == "" {
                    display.clear(Rgb565::BLACK.into()).unwrap();
                    y = height;
                    continue;
                }

                if y > 135 {
                    y = height;
                    display.clear(Rgb565::BLACK.into()).unwrap();
                }
                Text::new(&msg, Point::new(0, y), style)
                    .draw(display)
                    .unwrap();
                y += height;
            }
            display.clear(Rgb565::BLUE.into()).unwrap();
            thread::sleep(Duration::from_millis(1000));

            display.hard_reset(&mut delay::Ets).unwrap();
            backlight.set_low().unwrap();
        });

    Ok(tx)
}


pub struct DisplayDetails {
    pub buses: Vec<ArrivalTime>,
    pub battery: f32,
    pub wifi: u8,
}

pub fn start(
    busy: gpio::Gpio4<gpio::Unknown>,
    dc: gpio::Gpio16<gpio::Unknown>,
    rst: gpio::Gpio13<gpio::Unknown>,
    spi: spi::SPI2,
    sclk: gpio::Gpio18<gpio::Unknown>,
    sdo: gpio::Gpio19<gpio::Unknown>,
    cs: gpio::Gpio5<gpio::Unknown>,
) -> Result<mpsc::SyncSender<String>> {
    let config = <spi::config::Config as Default>::default().baudrate(26.MHz().into());

    println!("Setup eink display SPI interface");

    let mut spi_interface = spi::Master::<spi::SPI2, _, _, _, _>::new(
        spi,
        spi::Pins {
            sclk,
            sdo,
            sdi: Option::<gpio::Gpio21<gpio::Unknown>>::None,
            cs: Option::<gpio::Gpio5<gpio::Unknown>>::None,
        },
        config,
    )?;

    let mut eink = EPD3in7::new(
        &mut spi_interface,
        cs.into_output()?,
        busy.into_input()?,
        dc.into_output()?,
        rst.into_output()?,
        &mut delay::FreeRtos,
    )?;

    eink.set_lut(&mut spi_interface, Some(RefreshLut::Quick))?;

    let (tx, rx) = mpsc::sync_channel::<String>(5);

    let _ = std::thread::Builder::new()
        .stack_size(4_000)
        .spawn(move || {
            let mut display = Box::new(Display3in7::default());
            display.clear(Color::Black).unwrap();
            eink.update_and_display_frame(
                &mut spi_interface,
                display.buffer(),
                &mut delay::FreeRtos,
            )
            .unwrap();
            display.clear(Color::White).unwrap();
            eink.update_and_display_frame(
                &mut spi_interface,
                display.buffer(),
                &mut delay::FreeRtos,
            )
            .unwrap();
            display.set_rotation(DisplayRotation::Rotate90);

            let black_font = MonoTextStyle::new(&FONT_9X18_BOLD, Color::Black);

            let height = black_font.font.character_size.height as i32;
            let mut y = height;

            let bus = &ImageRaw::new_binary(include_bytes!("../../icons/Bus.raw"), 30);
            let bus2 = &ImageRaw::new_binary(include_bytes!("../../icons/Bus2.raw"), 30);
            let work = &ImageRaw::new_binary(include_bytes!("../../icons/Work.raw"), 30);
            let school = &ImageRaw::new_binary(include_bytes!("../../icons/School.raw"), 30);
            let batt2 = &ImageRaw::new_binary(include_bytes!("../../icons/Batt2.raw"), 30);

            for msg in rx {
                println!("Display: {}", msg);
                if msg.is_empty() {
                    display.clear(Color::White).unwrap();
                    y = height;
                    bus.draw(&mut display.translated(Point::new(100,200)).color_converted()).unwrap();
                    bus2.draw(&mut display.translated(Point::new(140,200)).color_converted()).unwrap();
                    work.draw(&mut display.translated(Point::new(180,200)).color_converted()).unwrap();
                    school.draw(&mut display.translated(Point::new(220,200)).color_converted()).unwrap();
                    batt2.draw(&mut display.translated(Point::new(260,200)).color_converted()).unwrap();
                    continue;
                } else if msg == "*" {
                    eink.update_and_display_frame(
                        &mut spi_interface,
                        display.buffer(),
                        &mut delay::FreeRtos,
                    )
                    .unwrap();
                    continue;
                }

                Text::new(&msg, Point::new(0, y), black_font)
                    .draw(&mut *display)
                    .unwrap();
                y += height;
            }

            display.clear(Color::White).unwrap();
            eink.set_lut(&mut spi_interface, Some(RefreshLut::Full))
                .unwrap();
            eink.update_and_display_frame(
                &mut spi_interface,
                display.buffer(),
                &mut delay::FreeRtos,
            )
            .unwrap();
            eink.sleep(&mut spi_interface, &mut delay::FreeRtos)
                .unwrap();
        });

    Ok(tx)
}
