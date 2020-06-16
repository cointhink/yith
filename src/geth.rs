use crate::errors;
use bs58;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionReceipt {
    pub status: String,
    pub cumulative_gas_used: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Client {
    url: String,
}

impl Client {
    pub fn build_infura(project_id: &str) -> Client {
        let infura_api = "https://mainnet.infura.io/v3";
        Client {
            url: format!("{}/{}", infura_api, project_id),
        }
    }

    pub fn rpc_str(
        &self,
        method: &str,
        params: ParamTypes,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let result = rpc(&self.url, method, params);
        match result {
            Ok(rpc_result) => match rpc_result.part {
                RpcResultTypes::Error(e) => Err(errors::MainError::build_box(e.error.message)),
                RpcResultTypes::Result(r) => {
                    let str_ret = match r.result {
                        ResultTypes::String(s) => s,
                        _ => "-bad response".to_string(),
                    };
                    Ok(str_ret)
                }
            },
            Err(e) => Err(e),
        }
    }

    pub fn rpc(
        &self,
        method: &str,
        params: ParamTypes,
    ) -> Result<JsonRpcResult, Box<dyn std::error::Error>> {
        rpc(&self.url, method, params)
    }

    pub fn last_block(&self) -> u32 {
        let blk_num_str = self
            .rpc_str("eth_blockNumber", ParamTypes::Single(("".to_string(),)))
            .unwrap();
        u32::from_str_radix(&blk_num_str[2..], 16).unwrap()
    }

    pub fn nonce(&self, addr: &str) -> Result<u32, Box<dyn error::Error>> {
        let params = (addr.to_string(), "latest".to_string());
        let tx_count_str =
            self.rpc_str("eth_getTransactionCount", ParamTypes::InfuraSingle(params))?;
        Ok(u32::from_str_radix(&tx_count_str[2..], 16).unwrap())
    }
}

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
    pub part: RpcResultTypes,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RpcResultTypes {
    Error(ErrorRpc),
    Result(ResultRpc),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResultTypes {
    String(String),
    TransactionReceipt(TransactionReceipt),
    Null,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResultRpc {
    pub result: ResultTypes,
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
    let result = client.post(url).json(&jrpc).send();
    match result {
        Ok(res) => {
            let json = res.text().unwrap();
            println!("geth: {}", json);
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
pub fn ethgasstation_fast() -> u64 {
    let gas_prices = ethgasstation();
    (gas_prices.fast as f64 * 100_000_000u64 as f64) as u64
}
