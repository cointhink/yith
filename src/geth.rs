use bs58;
use rand::Rng;
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
    Single(SingleParam),
    Infura(JsonInfuraRpcParam),
    InfuraSingle(InfuraSingleParam),
}

pub type JsonRpcParam = HashMap<String, String>;
pub type SingleParam = (String,);
pub type InfuraSingleParam = (String, String);
pub type JsonInfuraRpcParam = (JsonRpcParam, Option<String>);

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResult {
    pub jsonrpc: String,
    pub id: String,
    #[serde(flatten)]
    pub part: ResultTypes,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResultTypes {
    Error(ErrorRpc),
    Result(ResultRpc),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResultRpc {
    pub result: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorRpc {
    pub error: ErrorDetailRpc,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetailRpc {
    pub code: i32,
    pub message: String,
}

pub fn rpc(
    url: &str,
    method: &str,
    params: ParamTypes,
) -> Result<JsonRpcResult, Box<dyn std::error::Error>> {
    let jrpc = JsonRpc {
        jsonrpc: "2.0".to_string(),
        id: gen_id(),
        method: method.to_string(),
        params: params,
    };
    let client = reqwest::blocking::Client::new();
    println!("{}", url);
    println!("{}", serde_json::to_string(&jrpc).unwrap());
    let result = client.post(url).json(&jrpc).send();
    match result {
        Ok(res) => {
            let json = res.text().unwrap();
            let rpc_result = serde_json::from_str::<JsonRpcResult>(&json).unwrap();
            Ok(rpc_result)
        }
        Err(e) => Err(Box::new(e)),
    }
}

pub fn gen_id() -> String {
    let mut pad = [0u8; 6];
    rand::thread_rng().fill(&mut pad);
    bs58::encode(pad).into_string()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthGasStationResult {
    pub fast: f32,
    pub fastest: f32,
    pub safe_low: f32,
    pub average: f32,
}

pub fn ethgasstation() -> EthGasStationResult {
    let url = "https://ethgasstation.info/api/ethgasAPI.json";
    let client = reqwest::blocking::Client::new();
    let result = client.get(url).send().unwrap();
    result.json::<EthGasStationResult>().unwrap()
}
