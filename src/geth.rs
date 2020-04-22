use crate::time;
use crate::eth;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use bs58;

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
    let params_json = serde_json::to_string(&params).unwrap();
    let jrpc = JsonRpc {
        jsonrpc: "2.0".to_string(),
        id: mk_id(&params_json),
        method: method.to_string(),
        params: params,
    };
    let client = reqwest::blocking::Client::new();
    println!("{}", url);
    println!("{}", serde_json::to_string(&jrpc).unwrap());
    client.post(url).json(&jrpc).send()
}

pub fn mk_id(data: &str) -> String {
    let mut uniq = Vec::<u8>::new();
    uniq.append(&mut time::now_bytes()[0..4].to_vec());
    uniq.append(&mut eth::hash_msg(&data.as_bytes().to_vec())[0..4].to_vec());
    bs58::encode(uniq).into_string()
}
