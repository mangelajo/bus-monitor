pub mod emtmadrid;
pub mod peripherals;
pub mod wifi;

use std::ptr;
use std::thread;
use std::time::*;

use log::*;

use anyhow::Result;

use crate::emtmadrid::ArrivalTime;
use crate::emtmadrid::EMTMadridClient;

use crate::peripherals::display::DisplayMessage;

use esp_idf_svc::sntp;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys::time_t;
//use time::macros::offset;
use time::OffsetDateTime;

use esp_idf_svc::sntp::SyncStatus;

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

    display.send(DisplayMessage::Message(
        "EMTMadrid connecting...".to_string(),
    ))?;
    display.send(DisplayMessage::Update)?;

    let client = EMTMadridClient::new_from_email(EMT_USER, EMT_PASS)?;

    display.send(DisplayMessage::Message("EMTMadrid Login OK".to_string()))?;
    display.send(DisplayMessage::Update)?;

    /* Unsafe section is used since it's required, if you're using C functions and datatypes */
    unsafe {
        let sntp = sntp::EspSntp::new_default()?;
        info!("SNTP initialized, waiting for status!");

        while sntp.get_sync_status() != SyncStatus::Completed {}

        info!("SNTP status received!");

        let timer: *mut time_t = ptr::null_mut();

        let mut timestamp = esp_idf_sys::time(timer);

        let mut actual_date = OffsetDateTime::from_unix_timestamp(timestamp as i64)?
            // .to_offset(offset!(+2))
            .date();

        display.send(DisplayMessage::Message(actual_date.to_string()))?;
        display.send(DisplayMessage::Update)?;
    }

    for _n in 1..200 {
        let mut arrivals = Vec::<ArrivalTime>::new();

        match client.get_arrival_times("874") {
            Ok(arr) => {
                arrivals.append(&mut arr.clone());
            }
            Err(e) => {
                error!("Error getting arrival times: {}", e);
            }
        }

        match client.get_arrival_times("1455") {
            Ok(arr) => {
                arrivals.append(&mut arr.clone());
            }
            Err(e) => {
                error!("Error getting arrival times: {}", e);
            }
        }

        info!("Arrivals: {:?}", arrivals);
        display.send(DisplayMessage::Arrivals(arrivals))?;

        thread::sleep(Duration::from_millis(5000));
    }

    drop(display);
    thread::sleep(Duration::from_millis(5000));

    unsafe {
        esp_deep_sleep_start();
    }
    Ok(())
}

/*

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
} */
