pub mod peripherals;
pub mod emtmadrid;

use embedded_svc::wifi::*;
use esp_idf_svc::wifi::*;
use esp_idf_svc::{netif::EspNetifStack, nvs::EspDefaultNvs, sysloop::EspSysLoopStack};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use emtmadrid::EMTMadridClient;

use log::*;

use anyhow::{bail, Result};

const SSID: &str = env!("RUST_ESP32_STD_DEMO_WIFI_SSID");
const PASS: &str = env!("RUST_ESP32_STD_DEMO_WIFI_PASS");

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

    let netif_stack = Arc::new(EspNetifStack::new()?);
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
    let default_nvs = Arc::new(EspDefaultNvs::new()?);

    let mut _wifi = setup_wifi(
        netif_stack.clone(),
        sys_loop_stack.clone(),
        default_nvs.clone(),
    )?;

    display.send("EMTMadrid connecting...".to_string())?;

    let client = EMTMadridClient::new_from_email(EMT_USER, EMT_PASS)?;

    display.send("EMTMadrid Login OK".to_string())?;

    for _n in 1..10 {
        let arrivals = client.get_arrival_times("874")?;
        display.send(String::from(""))?;

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
        display.send(String::from("*"))?;
        thread::sleep(Duration::from_millis(60000));
    }

    drop(display);
    thread::sleep(Duration::from_millis(30000));

    unsafe {
        esp_deep_sleep_start();
    }
    Ok(())
}

fn setup_wifi(
    netif_stack: Arc<EspNetifStack>,
    sys_loop_stack: Arc<EspSysLoopStack>,
    default_nvs: Arc<EspDefaultNvs>,
) -> Result<Box<EspWifi>, anyhow::Error> {
    let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);

    info!("Wifi created, about to scan");

    let ap_infos = wifi.scan()?;

    let ours = ap_infos.into_iter().find(|a| a.ssid == SSID);

    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            SSID, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            SSID
        );
        None
    };

    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: SSID.into(),
            password: PASS.into(),
            channel,
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: "aptest".into(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    ))?;

    info!("Wifi configuration set, about to get status");

    wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
        .map_err(|e| anyhow::anyhow!("Unexpected Wifi status: {:?}", e))?;

    let status = wifi.get_status();

    if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
        ApStatus::Started(ApIpStatus::Done),
    ) = status
    {
        info!("Wifi connected to {} with IP {}", SSID, ip_settings.ip);
    } else {
        bail!("Unexpected Wifi status: {:?}", status);
    }

    Ok(wifi)
}