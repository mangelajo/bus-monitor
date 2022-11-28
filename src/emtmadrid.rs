use std;

pub struct EMTMadridClient<'a> {
    access_token: Option<String>,
    email: &'a str,
    password: &'a str,
}

pub struct ArrivalTime {
    pub line: String,
    pub destination: String,
    pub arrival_time: u64,
}


impl EMTMadridClient<'_> {
    pub fn new_from_email<'a>(
        email: &'a str,
        password: &'a str,
    ) -> anyhow::Result<EMTMadridClient<'a>> {
        let mut client = EMTMadridClient {
            access_token: None,
            email: email,
            password: password,
        };

        client.login()?;

        Ok(client)
    }

    pub fn new_with_token<'a>(
        token: &'a str,
        email: &'a str,
        password: &'a str,
    ) -> anyhow::Result<EMTMadridClient<'a>> {
        let client = EMTMadridClient {
            access_token: Some(String::from(token)),
            email: email,
            password: password,
        };

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
            return Ok(());
        } else {
            dbg!(&v);
        }
        Err(anyhow::anyhow!("Error logging in, access token not found"))
    }

    pub fn get_arrival_times(&self, stop_id: &str) -> anyhow::Result<Vec<ArrivalTime>> {
        use embedded_svc::http::client::*;
        use embedded_svc::io;
        use esp_idf_svc::http::client::*;
        use serde_json::Value;

        let url = format!(
            "https://openapi.emtmadrid.es/v1/transport/busemtmad/stops/{}/arrives/",
            stop_id
        );

        let mut client = EspHttpClient::new(&EspHttpClientConfiguration {
            crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),

            ..Default::default()
        })?;

        let mut request = client.post(&url)?;

        request.set_header("accessToken", self.access_token.as_ref().unwrap());
        request.set_header("Content-Type", "application/json");

        let mut response = request
            .send_str(r#"{"Text_EstimationsRequired_YN" : "Y"}"#)?
            .submit()?;

        let mut body = [0_u8; 3048];

        let (body, _) = io::read_max(response.reader(), &mut body)?;

        let v: Value = serde_json::from_slice(body)?;

        let mut arrival_times = Vec::new();

        if let Value::Array(arrivals) = &v["data"][0]["Arrive"] {
            for arrival in arrivals {
                let destination = &arrival["destination"].as_str().unwrap();
                let line = &arrival["line"].as_str().unwrap();
                let estimate_arrival_secs = &arrival["estimateArrive"].as_u64().unwrap();

                arrival_times.push(ArrivalTime {
                    line: line.to_string(),
                    destination: destination.to_string(),
                    arrival_time: *estimate_arrival_secs,
                });
            }
        } else {
            return Err(anyhow::anyhow!(
                "Error getting arrivals, arrivals not found"
            ));
        }

        Ok(arrival_times)
    }
}

