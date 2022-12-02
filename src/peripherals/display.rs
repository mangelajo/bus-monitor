use anyhow::Result;

use crate::get_time;

use embedded_graphics::image::ImageRaw;
use embedded_graphics::mono_font::iso_8859_1::{FONT_5X7, FONT_9X18_BOLD};
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle};
use embedded_graphics::text::*;

use esp_idf_hal::prelude::*;
use esp_idf_hal::{delay, gpio, spi};

#[cfg(feature = "ttgo")]
use st7789;

//#[cfg(esp32s3)]
use epd_waveshare::{color::*, epd3in7::*, prelude::*};

use std;
use std::sync::mpsc;
use std::time::Duration;

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

#[derive(Debug)]
pub enum DisplayMessage {
    Arrivals(Vec<ArrivalTime>),
    Message(String),
    Battery(f32),
    WiFi(f32),
    Clear,
    Update,
}

struct GraphicAssets<'a> {
    battery: [ImageRaw<'a, BinaryColor>; 5],
    school: ImageRaw<'a, BinaryColor>,
    work: ImageRaw<'a, BinaryColor>,
    bus: ImageRaw<'a, BinaryColor>,
    font: MonoTextStyle<'a, Color>,
    font_striket: MonoTextStyle<'a, Color>,
    mini_font: MonoTextStyle<'a, Color>,
}

fn load_graphic_assets<'a>() -> GraphicAssets<'a> {
    GraphicAssets::<'a> {
        battery: [
            ImageRaw::new_binary(include_bytes!("../../icons/Batt0.raw"), 30),
            ImageRaw::new_binary(include_bytes!("../../icons/Batt1.raw"), 30),
            ImageRaw::new_binary(include_bytes!("../../icons/Batt2.raw"), 30),
            ImageRaw::new_binary(include_bytes!("../../icons/Batt3.raw"), 30),
            ImageRaw::new_binary(include_bytes!("../../icons/Batt4.raw"), 30),
        ],
        school: ImageRaw::new_binary(include_bytes!("../../icons/School.raw"), 30),
        work: ImageRaw::new_binary(include_bytes!("../../icons/Work.raw"), 30),
        bus: ImageRaw::new_binary(include_bytes!("../../icons/Bus2.raw"), 30),
        font: MonoTextStyle::new(&FONT_9X18_BOLD, Color::Black),
        font_striket: MonoTextStyleBuilder::new()
            .font(&FONT_9X18_BOLD)
            .text_color(Color::Black)
            .strikethrough_with_color(Color::Black)
            .background_color(Color::White)
            .build(),
        mini_font: MonoTextStyleBuilder::new()
            .font(&FONT_5X7)
            .text_color(Color::Black)
            .background_color(Color::White)
            .build(),
    }
}

pub fn start(
    busy: gpio::Gpio4<gpio::Unknown>,
    dc: gpio::Gpio16<gpio::Unknown>,
    rst: gpio::Gpio13<gpio::Unknown>,
    spi: spi::SPI2,
    sclk: gpio::Gpio18<gpio::Unknown>,
    sdo: gpio::Gpio19<gpio::Unknown>,
    cs: gpio::Gpio5<gpio::Unknown>,
) -> Result<mpsc::SyncSender<DisplayMessage>> {
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

    let (tx, rx) = mpsc::sync_channel::<DisplayMessage>(5);

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

            let assets = load_graphic_assets();

            let font_height = assets.font.font.character_size.height as i32;
            let mut y = font_height;

            for msg in rx {
                match msg {
                    DisplayMessage::Clear => {
                        clear_display(&mut *display, &assets).unwrap();
                        y = font_height;
                        continue;
                    }

                    DisplayMessage::Update => {
                        eink.update_and_display_frame(
                            &mut spi_interface,
                            display.buffer(),
                            &mut delay::FreeRtos,
                        )
                        .unwrap();
                        continue;
                    }

                    DisplayMessage::Message(msg) => {
                        Text::new(&msg, Point::new(0, y), assets.font)
                            .draw(&mut *display)
                            .unwrap();
                        y += font_height;
                    }

                    DisplayMessage::Arrivals(arrivals) => {
                        draw_arrivals(&mut *display, &assets, &arrivals).unwrap();
                        draw_buses(&mut *display, &assets, &arrivals).unwrap();
                    }

                    others => {
                        println!("Display: {:?}", others);
                    }
                }
            }

            // If the msg channel is closed, we should shut down the display and put it to sleep
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

fn clear_display<D>(display: &mut D, assets: &GraphicAssets) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Color>,
{
    display.clear(Color::White)?;
    let font_height = assets.font.font.character_size.height as i32;
    let font_width = assets.font.font.character_size.width as i32;

    let display_height = display.bounding_box().size.height as i32 - 1;
    let display_width = display.bounding_box().size.width as i32 - 1;
    let batt_height = assets.battery[4].bounding_box().size.height as i32;
    let batt_width = assets.battery[4].bounding_box().size.width as i32;

    let header = " STOP LINE                 TIME     ";

    Text::new(&header, Point::new(0, font_height), assets.font).draw(&mut *display)?;

    assets.school.draw(
        &mut display
            .translated(Point::new((header.len() as i32 * font_width).into(), 0))
            .color_converted(),
    )?;
    assets.work.draw(
        &mut display
            .translated(Point::new(
                ((header.len() as i32 + 7) * font_width).into(),
                0,
            ))
            .color_converted(),
    )?;
    assets.battery[4].draw(
        &mut display
            .translated(Point::new(
                display_width - batt_width,
                display_height - batt_height,
            ))
            .color_converted(),
    )?;

    let display_width = display.bounding_box().size.width as i32 - 1;
    let thin_stroke = PrimitiveStyle::with_stroke(Color::Black, 1);
    Line::new(
        Point::new(0, font_height + 2),
        Point::new(display_width, font_height + 2),
    )
    .into_styled(thin_stroke)
    .draw(&mut *display)?;

    let t = get_time().time();
    let clock = format!("{:02}:{:02}", t.hour(), t.minute());
    Text::new(
        &clock,
        Point::new(
            (display_width as i32 - font_width * 5 - 3) as i32,
            font_height - 4,
        ),
        assets.font,
    )
    .draw(&mut *display)?;

    Ok(())
}

struct LineInfo<'a> {
    name: &'a str,
    seconds_from_home: u32,
    seconds_to_school: u32,
    seconds_to_work: u32,
}

const SECONDS_TO_874: u32 = 5 * 60;
const SECONDS_TO_1455: u32 = 3 * 60;

const LINE_INFO: [LineInfo; 6] = [
    LineInfo {
        name: "31",
        seconds_from_home: SECONDS_TO_874,
        seconds_to_school: (4 + 4) * 60,
        seconds_to_work: (8 + 8) * 60,
    },
    LineInfo {
        name: "33",
        seconds_from_home: SECONDS_TO_874,
        seconds_to_school: (5 + 6) * 60,
        seconds_to_work: 0,
    },
    LineInfo {
        name: "36",
        seconds_from_home: SECONDS_TO_874,
        seconds_to_school: (5 + 1) * 60,
        seconds_to_work: 0,
    },
    LineInfo {
        name: "39",
        seconds_from_home: SECONDS_TO_874,
        seconds_to_school: (5 + 6) * 60,
        seconds_to_work: (7 + 7) * 60,
    },
    LineInfo {
        name: "65",
        seconds_from_home: SECONDS_TO_874,
        seconds_to_school: (4 + 4) * 60,
        seconds_to_work: (8 + 8) * 60,
    },
    LineInfo {
        name: "138",
        seconds_from_home: SECONDS_TO_1455,
        seconds_to_school: (6 + 6) * 60,
        seconds_to_work: (12 + 7) * 60,
    },
];

fn get_line_info(line: &str) -> &LineInfo {
    for l in LINE_INFO.iter() {
        if l.name == line {
            return l;
        }
    }
    &LineInfo {
        name: "??",
        seconds_from_home: 0,
        seconds_to_school: 0,
        seconds_to_work: 0,
    }
}

fn draw_arrivals<D>(
    display: &mut D,
    assets: &GraphicAssets,
    arrivals: &Vec<ArrivalTime>,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Color>,
{
    let font_height = assets.font.font.character_size.height as i32;
    let mut y = font_height * 2 + 2;

    for arrival in arrivals {
        let t_str = time_string(arrival);
        let t_now = get_time();
        let mut t_at_school = String::from("");
        let mut t_at_work = String::from("");
        let line_info = get_line_info(&arrival.line);

        if arrival.time < 19999 {
            if line_info.seconds_to_school != 0 {
                let school_time = arrival.time as u32 + line_info.seconds_to_school;
                let t = t_now + Duration::from_secs(school_time as u64);
                t_at_school = format!("{:02}:{:02}", t.hour(), t.minute());
            }

            if line_info.seconds_to_work != 0 {
                let work_time = arrival.time as u32 + line_info.seconds_to_work;
                let t = t_now + Duration::from_secs(work_time as u64);
                t_at_work = format!("{:02}:{:02}", t.hour(), t.minute());
            }
        }

        let line = format!(
            "{:>4} {:>3} {:15} {:5}   {:5}   {:5}",
            arrival.stop, arrival.line, arrival.destination, t_str, t_at_school, t_at_work,
        );

        if arrival.time > line_info.seconds_from_home.into() {
            Text::new(&line, Point::new(0, y), assets.font).draw(&mut *display)?;
        } else {
            Text::new(&line, Point::new(0, y), assets.font_striket).draw(&mut *display)?;
        }

        y += font_height;
        if y >= display.bounding_box().size.height as i32
            - (assets.bus.bounding_box().size.height as i32) * 2
        {
            Text::new(&"...", Point::new(0, y - 5), assets.font).draw(&mut *display)?;
            break;
        }
    }
    Ok(())
}

fn draw_buses<D>(
    display: &mut D,
    assets: &GraphicAssets,
    arrivals: &Vec<ArrivalTime>,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Color>,
{
    let display_height = display.bounding_box().size.height as i32 - 1;
    let display_width = display.bounding_box().size.width as i32 - 1 - (30 * 3); // leave room for bus and , stop and battery icon
    let bus_height = assets.bus.bounding_box().size.height as i32;

    for arrival in arrivals {
        let max_time = 12 * 60 as i32;
        let t = arrival.time as i32;
        if t > max_time {
            continue;
        }
        let x = display_width - (t * display_width / max_time) as i32;

        assets.bus.draw(
            &mut display
                .translated(Point::new(x, display_height - bus_height))
                .color_converted(),
        )?;
        Text::new(
            &format!("{}", &arrival.line),
            Point::new(x + 7, display_height - bus_height),
            assets.mini_font,
        )
        .draw(&mut *display)?;
    }
    Ok(())
}

fn time_string(arrival: &ArrivalTime) -> String {
    if arrival.time == 0 {
        return String::from(">>>>>>>");
    } else if arrival.time > 19999 {
        return String::from("      ");
    }

    let time_m = arrival.time / 60;
    let time_s = arrival.time % 60;
    return format!("{:>2}m {:02}s", time_m, time_s);
}
