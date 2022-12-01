use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};
use embedded_svc::wifi::*;
use esp_idf_svc::wifi::*;
use esp_idf_svc::{netif::EspNetifStack, nvs::EspDefaultNvs, sysloop::EspSysLoopStack};
use log::*;

const SSID: &str = env!("RUST_ESP32_STD_DEMO_WIFI_SSID");
const PASS: &str = env!("RUST_ESP32_STD_DEMO_WIFI_PASS");

pub fn setup_wifi() -> Result<Box<EspWifi>, anyhow::Error> {
    let netif_stack = Arc::new(EspNetifStack::new()?);
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
    let default_nvs = Arc::new(EspDefaultNvs::new()?);

    let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);
    /*
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
    */
    wifi.set_configuration(&Configuration::Client(
        //Mixed
        ClientConfiguration {
            ssid: SSID.into(),
            password: PASS.into(),
            channel: None, /* channel */
            ..Default::default()
        },
        /*       AccessPointConfiguration {
            ssid: "busmonitor".into(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
        */
    ))?;

    info!("Wifi configuration set, about to get status");

    wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
        .map_err(|e| anyhow::anyhow!("Unexpected Wifi status: {:?}", e))?;

    let status = wifi.get_status();

    if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
        ApStatus::Stopped, //ApStatus::Started(ApIpStatus::Done),
    ) = status
    {
        info!("Wifi connected to {} with IP {}", SSID, ip_settings.ip);
    } else {
        bail!("Unexpected Wifi status: {:?}", status);
    }

    Ok(wifi)
}
