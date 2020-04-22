use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpc {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: Vec<JsonRpcParam>,
}

pub type JsonRpcParam = HashMap<String, String>;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonInfuraRpc {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: JsonInfuraRpcParam,
}
pub type JsonInfuraRpcParam = (JsonRpcParam, Option<String>);

#[allow(dead_code)]
pub fn rpc(
    config: &crate::config::Config,
    url: &str,
    method: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let jrpc = JsonRpc {
        jsonrpc: "2.0".to_string(),
        id: "12".to_string(),
        method: method.to_string(),
        params: Vec::new(),
    };
    let client = reqwest::blocking::Client::new();
    let resp = client.post(url).json(&jrpc).send()?;
    //let resp = client.post(url).body + '_(method).send().await?;
    println!("rpc {:#?} ", resp);
    let body = resp.json::<HashMap<String, String>>()?;
    println!("body {:#?} ", body);
    Ok(())
}
