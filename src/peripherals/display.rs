use std::time::Duration;
use std::thread;

use embedded_graphics::mono_font::iso_8859_5::FONT_6X12;

use esp_idf_hal::prelude::*;
use esp_idf_hal::spi;
use anyhow::Result;


use embedded_graphics::pixelcolor::*;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::prelude::*;
use embedded_graphics::text::*;
//use embedded_graphics::primitives::*;

use esp_idf_hal::{gpio, delay};
//use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::OutputPin;

use display_interface_spi::SPIInterfaceNoCS;

use st7789;

use std;
use std::sync::mpsc;

pub fn start(
        backlight: gpio::Gpio4<gpio::Unknown>,
        dc: gpio::Gpio16<gpio::Unknown>,
        rst: gpio::Gpio23<gpio::Unknown>,
        spi: spi::SPI2,
        sclk: gpio::Gpio18<gpio::Unknown>,
        sdo: gpio::Gpio19<gpio::Unknown>,
        cs: gpio::Gpio5<gpio::Unknown>,
    ) -> Result<mpsc::SyncSender<String>> {

        // Speed here could be faster, but the touch screen controller
        // is on the same SPI bus
        let config = <spi::config::Config as Default>::default()
        .baudrate(26.MHz().into());
    
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

        let mut display = st7789::ST7789::new(di,rst.into_output()?, 240, 320);
    
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
      
        let _ = std::thread::Builder::new().stack_size(16_000).spawn(move || {
           
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
                    continue
                }

                if y>135 {
                    y = height;
                    display.clear(Rgb565::BLACK.into()).unwrap();
                }
                 Text::new(&msg, Point::new(0, y), style)
                    .draw(display).unwrap();
                y += height;
                
            }
            display.clear(Rgb565::BLUE.into()).unwrap(); 
            thread::sleep(Duration::from_millis(1000));
           
            display.hard_reset(&mut delay::Ets).unwrap();
            backlight.set_low().unwrap();
        }
    );

    Ok(tx)
}
