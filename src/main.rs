
pub mod display;

pub use crate::display::Display;

use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use embedded_svc::wifi::*;
use esp_idf_svc::wifi::*;
use std::{sync::Arc, thread, time::*};
use esp_idf_svc::netif::EspNetifStack;
use esp_idf_svc::sysloop::EspSysLoopStack;
use esp_idf_svc::nvs::EspDefaultNvs;
use log::*;

/*
use esp_idf_hal::adc;
use esp_idf_hal::delay;
use esp_idf_hal::gpio;
use esp_idf_hal::i2c;
use esp_idf_hal::spi;
use esp_idf_hal::gpio;

*/
use esp_idf_hal::prelude::*;


use anyhow::{Result, bail};

const SSID: &str = env!("RUST_ESP32_STD_DEMO_WIFI_SSID");
const PASS: &str = env!("RUST_ESP32_STD_DEMO_WIFI_PASS");

fn main() -> Result<()>  {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    let mut display = Display::new(pins.gpio15, // backlight
                                   pins.gpio4,  // dc
                                   pins.gpio22, // rst
                                   peripherals.spi3,
                                   pins.gpio18,  // sclk
                                   pins.gpio23,  // sdo
                                   pins.gpio19,
                                   pins.gpio5,   // cs
                                  )?;

    display.paint()?;
    display.draw_text()?;

    let netif_stack = Arc::new(EspNetifStack::new()?);
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
    let default_nvs = Arc::new(EspDefaultNvs::new()?);

    let mut _wifi = setup_wifi(
        netif_stack.clone(),
        sys_loop_stack.clone(),
        default_nvs.clone(),
    )?;

    println!("Hello, world!");

    loop {
        println!("Just looping!");
        thread::sleep(Duration::from_secs(1));
    }

    //Ok(())
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