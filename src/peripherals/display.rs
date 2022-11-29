use std::thread;
use std::time::Duration;

//use embedded_graphics::mono_font::iso_8859_1::FONT_6X12;
use anyhow::Result;
use embedded_graphics::mono_font::iso_8859_1::FONT_10X20;
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::*;

use esp_idf_hal::{delay, gpio};
//use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::OutputPin;

use display_interface_spi::SPIInterfaceNoCS;

#[cfg(feature = "ttgo")]
use st7789;

//#[cfg(esp32s3)]
use epd_waveshare::{color::*, epd3in7::*, prelude::*};

use std;
use std::sync::mpsc;

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
    // Speed here could be faster, but the touch screen controller
    // is on the same SPI bus
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

pub fn start(
    busy: gpio::Gpio4<gpio::Unknown>,
    dc: gpio::Gpio16<gpio::Unknown>,
    rst: gpio::Gpio13<gpio::Unknown>,
    spi: spi::SPI2,
    sclk: gpio::Gpio18<gpio::Unknown>,
    sdo: gpio::Gpio19<gpio::Unknown>,
    cs: gpio::Gpio5<gpio::Unknown>,
) -> Result<mpsc::SyncSender<String>> {
    // Speed here could be faster, but the touch screen controller
    // is on the same SPI bus
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
/* 
    let mut eink = Epd5in65f::new(
        &mut spi_interface,
        cs.into_output()?,
        busy.into_input()?,
        dc.into_output()?,
        rst.into_output()?,
        &mut delay::FreeRtos,
    )?;
*/
    let mut eink = EPD3in7::new(
        &mut spi_interface,
        cs.into_output()?,
        busy.into_input()?,
        dc.into_output()?,
        rst.into_output()?,
        &mut delay::FreeRtos,
    )?;


    let (tx, rx) = mpsc::sync_channel::<String>(5);

    let _ = std::thread::Builder::new()
        .stack_size(4_000)
        .spawn(move || {
            
            let mut display = Box::new(Display3in7::default());
            display.clear(Color::Black).unwrap();
            eink.update_and_display_frame(&mut spi_interface, display.buffer(), &mut delay::FreeRtos)
            .unwrap();
            display.clear(Color::White).unwrap();
            eink.update_and_display_frame(&mut spi_interface, display.buffer(), &mut delay::FreeRtos)
            .unwrap();
            display.set_rotation(DisplayRotation::Rotate90);
            
             
            let black_font = MonoTextStyle::new(&FONT_10X20, Color::Black);
    
            let height = 20;
            let mut y = height;

            for msg in rx {
                println!("Display: {}", msg);
                if msg == "" {
                    display.clear(Color::White).unwrap();
                    y = height;
                    continue;
                } else if msg == "*" {
                    eink.update_and_display_frame(&mut spi_interface, display.buffer(), &mut delay::FreeRtos)
                        .unwrap();
                    continue;
                }

                Text::new(&msg, Point::new(0, y), black_font)
                    .draw(&mut *display)
                    .unwrap();
                y += height;
            }

            display.clear(Color::White).unwrap();
             
            eink.update_and_display_frame(&mut spi_interface, display.buffer(), &mut delay::FreeRtos)
                        .unwrap();
            
        });

    Ok(tx)
}
