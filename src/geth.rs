use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpc {
    jsonrpc: String,
    id: String,
    method: String,
    params: Vec<JsonRpcParam>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcParam {
    key: String,
    value: String,
}

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
