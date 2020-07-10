use crate::geth;
use crate::{http_error, http_info};
use reqwest::blocking::RequestBuilder;
use reqwest::{StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;

pub struct LoggingClient {
    client: reqwest::blocking::Client,
}

#[derive(Debug)]
pub enum Verb {
    Get,
    Post,
}

impl LoggingClient {
    pub fn new(client: reqwest::blocking::Client) -> LoggingClient {
        LoggingClient { client: client }
    }

    pub fn get(&self, url: &str) -> LoggingBuilder {
        self.method(Verb::Get, url)
    }

    pub fn post(&self, url: &str) -> LoggingBuilder {
        self.method(Verb::Post, url)
    }

    pub fn method(&self, verb: Verb, url_str: &str) -> LoggingBuilder {
        let id = geth::gen_id();
        let url = Url::parse(url_str).unwrap();
        println!(
            "[{}] {:?} {} {}",
            id,
            verb,
            url.host_str().unwrap(),
            url.path()
        );
        let builder = match verb {
            Verb::Get => self.client.get(url),
            Verb::Post => self.client.post(url),
        };
        LoggingBuilder {
            id: id,
            verb: verb,
            url: url_str.to_string(),
            json: None,
            builder: builder,
        }
    }
}

pub struct LoggingBuilder {
    id: String,
    verb: Verb,
    url: String,
    json: Option<String>,
    builder: RequestBuilder,
}

impl LoggingBuilder {
    pub fn headers(self, headers: reqwest::header::HeaderMap) -> LoggingBuilder {
        let builder = self.builder.headers(headers);
        LoggingBuilder {
            id: self.id,
            verb: self.verb,
            url: self.url,
            json: self.json,
            builder: builder,
        }
    }
    pub fn json<T: Serialize + ?Sized>(self, object: &T) -> LoggingBuilder {
        let json = serde_json::to_string(object).unwrap();
        let builder = self.builder.json(object);
        LoggingBuilder {
            id: self.id,
            verb: self.verb,
            url: self.url,
            json: Some(json),
            builder: builder,
        }
    }
    pub fn send(self) -> reqwest::Result<LoggingResponse> {
        http_info!("[{}] {:?} {}", self.id, self.verb, self.url);
        if self.json.is_some() {
            http_info!("[{}] {} ", self.id, self.json.unwrap());
        }
        let resp = self.builder.send();
        match resp {
            Ok(r) => {
                let status = r.status();
                let text = r.text().unwrap();
                http_info!("[{}] {} {}", self.id, status, text);
                Ok(LoggingResponse {
                    url: self.url,
                    status: status,
                    text: text.to_string(),
                })
            }
            Err(e) => {
                http_error!("[{}] {:?}", self.id, e);
                Err(e)
            }
        }
    }
}

pub struct LoggingResponse {
    url: String,
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
    pub fn url(&self) -> &str {
        &self.url
    }
}
