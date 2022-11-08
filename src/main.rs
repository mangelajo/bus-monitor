pub mod peripherals;

use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use embedded_svc::wifi::*;
use esp_idf_svc::wifi::*;
use std::sync::Arc;
use std::time::Duration;
use std::thread;
use esp_idf_svc::{
    netif::EspNetifStack,
    sysloop::EspSysLoopStack,
    nvs::EspDefaultNvs
};

use log::*;

use anyhow::{Result, bail};

const SSID: &str = env!("RUST_ESP32_STD_DEMO_WIFI_SSID");
const PASS: &str = env!("RUST_ESP32_STD_DEMO_WIFI_PASS");

const EMT_USER: &str = env!("EMT_USER");
const EMT_PASS: &str = env!("EMT_PASS");

extern "C" {
    fn esp_deep_sleep_start() -> i32;
}

fn main() -> Result<()>  {
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

    for _n in 1..50 {
        let arrivals = client.get_arrivals()?;
        display.send(String::from(""))?;



        for arrival in arrivals {
            let time;
            if arrival.arrival_time == 0 {
                time = String::from(">>>>>>");
            } else if arrival.arrival_time > 9999 {
                time = String::from("      ");
            } else {
                let time_m = arrival.arrival_time / 60;
                let time_s = arrival.arrival_time % 60;
                time = format!("{}m {:02}s", time_m, time_s);
            }

            display.send(format!("{:3} {:15} {}", arrival.line, arrival.destination, time))?;
        }
        thread::sleep(Duration::from_millis(5000));

    }
    
    drop(display);
    thread::sleep(Duration::from_millis(2000));

    unsafe { esp_deep_sleep_start();}
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


struct EMTMadridClient <'a>{
    access_token: Option<String>,
    email: &'a str,
    password: &'a str,
}

struct ArrivalTime {
    line: String,
    destination: String,
    arrival_time: u64,
}

impl EMTMadridClient<'_> {
    pub fn new_from_email<'a>(email: &'a str, password: &'a str) -> anyhow::Result<EMTMadridClient<'a>> {
        
        let mut client = EMTMadridClient {
            access_token: None,
            email: email,
            password: password,
        };

        client.login()?;

        Ok(client)
    }

    pub fn login(&mut self) -> anyhow::Result<()> {
        use embedded_svc::http::client::*;
        use embedded_svc::io;
        use esp_idf_svc::http::client::*;
        use serde_json::Value;
    
        let url = String::from("https://openapi.emtmadrid.es/v1/mobilitylabs/user/login/");
    
        let mut client = EspHttpClient::new(&EspHttpClientConfiguration {
            crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
    
            ..Default::default()
        })?;
    
        let mut request = client.get(&url)?;
        
        request.set_header("email", self.email);
        request.set_header("password", self.password);
    
        let mut response = request.submit()?;
    
        let mut body = [0_u8; 3048];
    
        let (body, _) = io::read_max(response.reader(), &mut body)?;

        let v: Value = serde_json::from_slice(body)?;

        if let Value::String(token) = &v["data"][0]["accessToken"] {
          self.access_token = Some(token.clone());
          return Ok(())
        } else {
          dbg!(&v);
        }
        Err(anyhow::anyhow!("Error logging in, access token not found"))
    }

    pub fn get_arrivals(&self) -> anyhow::Result<Vec<ArrivalTime>> {
        use embedded_svc::http::client::*;
        use embedded_svc::io;
        use esp_idf_svc::http::client::*;
        use serde_json::Value;

        let mut arrivals_r:Vec<ArrivalTime> = Vec::new();
    
        let url = String::from("https://openapi.emtmadrid.es/v2/transport/busemtmad/stops/874/arrives/");
    
        let mut client = EspHttpClient::new(&EspHttpClientConfiguration {
            crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
    
            ..Default::default()
        })?;
    
        let mut request = client.post(&url)?;

        request.set_header("accessToken", self.access_token.as_ref().unwrap());
        request.set_header("Content-Type", "application/json");

        let mut response = request.send_str(r#"{"Text_EstimationsRequired_YN" : "Y"}"#)?.submit()?;
    
        let mut body = [0_u8; 3048];
    
        let (body, _) = io::read_max(response.reader(), &mut body)?;

        let v: Value = serde_json::from_slice(body)?;

        // dbg!(&v);

        if let Value::Array(arrivals) = &v["data"][0]["Arrive"] {
            for arrival in arrivals {
                let destination = &arrival["destination"].as_str().unwrap();
                let line = &arrival["line"].as_str().unwrap();
                let estimate_arrival_secs = &arrival["estimateArrive"].as_u64().unwrap();
                
                let arrival_time = ArrivalTime {
                    line: line.to_string(),
                    destination: destination.to_string(),
                    arrival_time: *estimate_arrival_secs,
                };
                arrivals_r.push(arrival_time);
            }   
          } else {
            return Err(anyhow::anyhow!("Error getting arrivals, arrivals not found"));
          }

        Ok(arrivals_r)
    }
}
