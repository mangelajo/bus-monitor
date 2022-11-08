use esp_idf_hal::prelude::*;
use esp_idf_hal::spi;
use embedded_hal::digital::v2::OutputPin;
use anyhow::Result;
use ili9341::{Ili9341, Orientation};
use std::{thread, time::*};

use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::*;
//use embedded_graphics::primitives::*;
use embedded_graphics::text::*;
use embedded_graphics::image::Image;

use tinytga::Tga;

use display_interface_spi;
use shared_bus;
use esp_idf_hal::{gpio, gpio::*, spi::SPI3};
use esp_idf_hal::delay;


pub struct Display {
    // I hope there's a better way of declaring this
    display: Ili9341<
                display_interface_spi::SPIInterface<
                esp_idf_hal::spi::Master<SPI3, Gpio18<esp_idf_hal::gpio::Unknown>, 
                                               Gpio23<esp_idf_hal::gpio::Unknown>,
                                               Gpio19<esp_idf_hal::gpio::Unknown>,
                                               Gpio5<esp_idf_hal::gpio::Unknown>>,
                                               Gpio4<esp_idf_hal::gpio::Output>,
                                               Gpio5<esp_idf_hal::gpio::Output>>,
               Gpio22<esp_idf_hal::gpio::Output>,
               >,
}

impl Display {
    pub fn new(
        backlight: gpio::Gpio15<gpio::Unknown>,
        dc: gpio::Gpio4<gpio::Unknown>,
        rst: gpio::Gpio22<gpio::Unknown>,
        spi: spi::SPI3,
        sclk: gpio::Gpio18<gpio::Unknown>,
        sdo: gpio::Gpio23<gpio::Unknown>,
        sdi: gpio::Gpio19<gpio::Unknown>,
        cs: gpio::Gpio5<gpio::Unknown>,
    ) -> Result<Self> {

        // Speed here could be faster, but the touch screen controller
        // is on the same SPI bus
        let config = <spi::config::Config as Default>::default()
        .baudrate(3.MHz().into());

        let mut backlight = backlight.into_output()?;
        backlight.set_low()?;

        let spi_interface = spi::Master::<spi::SPI3, _, _, _, _>::new(
            spi,
            spi::Pins {
                sclk,
                sdo,
                sdi: Some(sdi),
                cs: None,
            },
            config,
        )?;

     
        let di = display_interface_spi::SPIInterface::new(
            spi_interface,
            dc.into_output()?,
            cs.into_output()?,
        );

        let reset = rst.into_output()?;

        let display = ili9341::Ili9341::new(
            di,
            reset,
            &mut delay::Ets,
            Orientation::Landscape,
            ili9341::DisplaySize240x320,
        )
        .map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;


        Ok(Display{display:display})
    }

    pub fn paint(&mut self) -> Result<()> {

        let display = &mut self.display;
        display.clear(Rgb565::BLUE.into()).map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;
        

        Ok(())
    }

    pub fn draw_text(&mut self) -> Result<()> {
        let display = &mut self.display;
        let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
        let text = Text::new("Hello Rust!", Point::new(0,20), text_style);
        text.draw(display).map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;

        // Include an image from a local path as bytes

        let tga = Tga::from_slice(include_bytes!("../assets/test.tga")).unwrap();
        /* 
        let tga_patri = Tga::from_slice(include_bytes!("../assets/patri.tga")).unwrap();
        let tga_patri2 = Tga::from_slice(include_bytes!("../assets/patri2.tga")).unwrap();
        */
        let tga_marga = Tga::from_slice(include_bytes!("../assets/marga.tga")).unwrap();
        let tga_marga2 = Tga::from_slice(include_bytes!("../assets/marga2.tga")).unwrap();

        let image = Image::new(&tga, Point::zero());
        //let image_patri = Image::new(&tga_patri, Point::zero());
        //let image_patri2 = Image::new(&tga_patri2, Point::zero());
        let image_marga = Image::new(&tga_marga, Point::zero());
        let image_marga2 = Image::new(&tga_marga2, Point::zero());

        let mut translated_display = display.translated(Point::new(0, 0));

        image.draw(&mut translated_display).map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;
        loop {
            image_marga.draw(&mut translated_display).map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;
            thread::sleep(Duration::from_millis(500));
            image_marga2.draw(&mut translated_display).map_err(|e| anyhow::anyhow!("Display error: {:?}", e))?;
            thread::sleep(Duration::from_millis(500));
        }
        Ok(())
    }
}