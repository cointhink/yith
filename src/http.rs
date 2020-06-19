use crate::{http_error, http_info};
use reqwest::blocking::RequestBuilder;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;

pub struct LoggingClient {
    client: reqwest::blocking::Client,
}

impl LoggingClient {
    pub fn new(client: reqwest::blocking::Client) -> LoggingClient {
        LoggingClient { client: client }
    }

    pub fn get(&self, url: &str) -> LoggingBuilder {
        http_error!("{}", url);
        let builder = self.client.get(url);
        LoggingBuilder { builder: builder }
    }

    pub fn post(&self, url: &str) -> LoggingBuilder {
        http_info!("{}", url);
        let builder = self.client.post(url);
        LoggingBuilder { builder: builder }
    }
}

pub struct LoggingBuilder {
    builder: RequestBuilder,
}

impl LoggingBuilder {
    pub fn headers(self, headers: reqwest::header::HeaderMap) -> LoggingBuilder {
        let builder = self.builder.headers(headers);
        LoggingBuilder { builder: builder }
    }
    pub fn json<T: Serialize + ?Sized>(self, json: &T) -> LoggingBuilder {
        http_info!("{}", serde_json::to_string(json).unwrap());
        let builder = self.builder.json(json);
        LoggingBuilder { builder: builder }
    }
    pub fn send(self) -> reqwest::Result<LoggingResponse> {
        let resp = self.builder.send();
        match resp {
            Ok(r) => {
                let status = r.status();
                let text = r.text().unwrap();
                http_info!("{}", text);
                Ok(LoggingResponse {
                    text: text.to_string(),
                    status: status,
                })
            }
            Err(e) => Err(e),
        }
    }
}

pub struct LoggingResponse {
    text: String,
    status: StatusCode,
}

impl LoggingResponse {
    pub fn status(&self) -> StatusCode {
        self.status
    }
    pub fn json<T: DeserializeOwned>(&self) -> Result<T, Box<dyn std::error::Error>> {
        match serde_json::from_str::<T>(&self.text) {
            Ok(r) => Ok(r),
            Err(e) => Err(Box::new(e)),
        }
    }
    pub fn text(&self) -> reqwest::Result<&str> {
        Ok(&self.text)
    }
}
