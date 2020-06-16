use crate::errors;
use crate::eth;
use crate::geth;

pub struct Erc20 {}

impl Erc20 {
    pub fn allowance(
        client: geth::Client,
        private_key: &str,
        token_addr: &str,
        trusted_contract_addr: &str,
    ) -> Result<u128, Box<dyn std::error::Error>> {
        let pub_addr = format!("0x{}", eth::privkey_to_addr(private_key));
        let data = allowance_data(&pub_addr, trusted_contract_addr);
        let mut tx = geth::JsonRpcParam::new();
        tx.insert("to".to_string(), token_addr.to_string());
        tx.insert("data".to_string(), eth::hex(&data));
        let params = (tx.clone(), Some("latest".to_string()));
        match client.rpc_str("eth_call", geth::ParamTypes::Infura(params)) {
            Ok(tx) => {
                println!("{:?}", tx);
                Ok(u128::from_str_radix(&tx[2..], 16).unwrap())
            }
            Err(e) => Err(errors::MainError::build_box(e.to_string())),
        }
    }

    pub fn approve(
        client: geth::Client,
        private_key: &str,
        token_addr: &str,
        trusted_contract_addr: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let pub_addr = format!("0x{}", eth::privkey_to_addr(private_key));
        let data = approve_data(trusted_contract_addr, std::u128::MAX);
        let gas_price_fast = geth::ethgasstation_fast();
        let nonce = client.nonce(&pub_addr).unwrap();
        let mut token_addr_bytes = [0u8; 20];
        token_addr_bytes.copy_from_slice(&eth::dehex(token_addr)[..]);
        let tx = ethereum_tx_sign::RawTransaction {
            nonce: ethereum_types::U256::from(nonce),
            to: Some(ethereum_types::H160::from(token_addr_bytes)),
            value: ethereum_types::U256::zero(),
            gas_price: ethereum_types::U256::from(gas_price_fast),
            gas: ethereum_types::U256::from(310240),
            data: data,
        };
        let private_key = ethereum_types::H256::from_slice(&eth::dehex(private_key));
        let rlp_bytes = tx.sign(&private_key, &eth::ETH_CHAIN_MAINNET);
        let params = (eth::hex(&rlp_bytes),);
        let result = client.rpc_str("eth_sendRawTransaction", geth::ParamTypes::Single(params));
        match result {
            Err(e) => Err(e),
            Ok(tx) => {
                println!("GOOD TX {}", tx);
                Ok(true)
            }
        }
    }
}

fn allowance_data(owner_addr: &str, spender_addr: &str) -> Vec<u8> {
    let mut call = Vec::<u8>::new();
    let mut func = eth::hash_abi_sig("allowance(address,address)").to_vec();
    call.append(&mut func);
    let mut p1 = hex::decode(eth::encode_addr2(owner_addr)).unwrap();
    call.append(&mut p1);
    let mut p2 = hex::decode(eth::encode_addr2(spender_addr)).unwrap();
    call.append(&mut p2);
    call
}

fn approve_data(spender_addr: &str, amount: u128) -> Vec<u8> {
    let mut call = Vec::<u8>::new();
    let mut func = eth::hash_abi_sig("approve(address,uint256)").to_vec();
    call.append(&mut func);
    let mut p1 = hex::decode(eth::encode_addr2(spender_addr)).unwrap();
    call.append(&mut p1);
    let mut p2 = hex::decode(eth::encode_uint256(&amount.to_string())).unwrap();
    call.append(&mut p2);
    call
}
