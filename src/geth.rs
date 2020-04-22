use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpc {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: ParamTypes,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParamTypes {
    Standard(JsonRpcParam),
    Infura(JsonInfuraRpcParam),
}

pub type JsonRpcParam = HashMap<String, String>;
pub type JsonInfuraRpcParam = (JsonRpcParam, Option<String>);

pub fn rpc(
    url: &str,
    method: &str,
    params: ParamTypes,
) -> Result<reqwest::blocking::Response, reqwest::Error> {
    let jrpc = JsonRpc {
        jsonrpc: "2.0".to_string(),
        id: "12".to_string(),
        method: method.to_string(),
        params: params,
    };
    let client = reqwest::blocking::Client::new();
    println!("{}", url);
    println!("{}", serde_json::to_string(&jrpc).unwrap());
    client.post(url).json(&jrpc).send()
}
