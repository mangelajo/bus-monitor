pub mod emtmadrid;
pub mod peripherals;
pub mod wifi;

use std::ptr;
use std::thread;
use std::time::*;

use log::*;

use anyhow::Result;
use time::UtcOffset;

use crate::emtmadrid::ArrivalTime;
use crate::emtmadrid::EMTMadridClient;

use crate::peripherals::display::DisplayMessage;

use esp_idf_svc::sntp;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys::time_t;

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

    let sntp = sntp::EspSntp::new_default()?;
    display.send(DisplayMessage::Message(
        "Updating time via SNTP".to_string(),
    ))?;

    while sntp.get_sync_status() != SyncStatus::Completed {}

    display.send(DisplayMessage::Message("Time updated".to_string()))?;

    let actual_time = get_time();

    display.send(DisplayMessage::Message(actual_time.to_string()))?;
    display.send(DisplayMessage::Update)?;

    for _n in 1..200 {
        display.send(DisplayMessage::Clear)?;
        let arrivals = get_my_arrivals(&client);
        display.send(DisplayMessage::Arrivals(arrivals))?;
        display.send(DisplayMessage::Update)?;

        thread::sleep(Duration::from_millis(5000));
    }

    drop(display);
    thread::sleep(Duration::from_millis(5000));

    unsafe {
        esp_deep_sleep_start();
    }
    Ok(())
}

pub fn get_time() -> time::OffsetDateTime {
    let timestamp;
    unsafe {
        let timer: *mut time_t = ptr::null_mut();
        timestamp = esp_idf_sys::time(timer);
    }

    let actual_time = OffsetDateTime::from_unix_timestamp(timestamp as i64)
        .unwrap()
        .to_offset(UtcOffset::from_hms(1, 0, 0).unwrap());
    actual_time
}

fn get_my_arrivals(client: &EMTMadridClient) -> Vec<ArrivalTime> {
    let mut arrivals = Vec::<ArrivalTime>::new();
    match client.get_arrival_times("874") {
        Ok(arr) => {
            arrivals = arr.clone();
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
    arrivals.sort();
    arrivals
}
