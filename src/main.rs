pub mod emtmadrid;
pub mod peripherals;
pub mod wifi;

use std::thread;
use std::time::Duration;

use anyhow::Result;

use emtmadrid::EMTMadridClient;

use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

const EMT_USER: &str = env!("EMT_USER");
const EMT_PASS: &str = env!("EMT_PASS");

extern "C" {
    fn esp_deep_sleep_start() -> i32;
}

fn main() -> Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let display = peripherals::init()?;

    let mut _wifi = wifi::setup_wifi()?;

    display.send("EMTMadrid connecting...".to_string())?;
    display.send(String::from("*")).unwrap();

    let client = EMTMadridClient::new_from_email(EMT_USER, EMT_PASS)?;

    display.send("EMTMadrid Login OK".to_string())?;
    display.send(String::from("*")).unwrap();

    for _n in 1..200 {
        match client.get_arrival_times("874") {
            Ok(arrivals) => {
                display.send(String::from("")).unwrap();

                for arrival in arrivals {
                    let time;
                    if arrival.arrival_time == 0 {
                        time = String::from(">>>>>>");
                    } else if arrival.arrival_time > 19999 {
                        time = String::from("      ");
                    } else {
                        let time_m = arrival.arrival_time / 60;
                        let time_s = arrival.arrival_time % 60;
                        time = format!("{}m {:02}s", time_m, time_s);
                    }

                    display.send(format!(
                        "{:3} {:15} {}",
                        arrival.line, arrival.destination, time
                    ))?;
                }
                display.send(String::from("*")).unwrap();
            }
            Err(e) => {
                display.send(format!("Error: {}", e)).unwrap();
                display.send(String::from("*")).unwrap();
            }
        }

        thread::sleep(Duration::from_millis(5000));
    }

    drop(display);
    thread::sleep(Duration::from_millis(5000));

    unsafe {
        esp_deep_sleep_start();
    }
    Ok(())
}
