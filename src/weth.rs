use crate::errors;
use crate::eth;
use crate::geth;

pub struct Weth {}

pub enum Direction {
    Wrap,
    Unwrap,
}

impl Weth {
    const CONTRACT_ADDRESS: &'static str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";

    pub fn wrap(
        client: geth::Client,
        private_key: &str,
        direction: Direction,
        amount: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let pub_addr = format!("0x{}", eth::privkey_to_addr(private_key));
        let (data, value) = match direction {
            Direction::Unwrap => (withdraw_data(amount), ethereum_types::U256::zero()),
            Direction::Wrap => (
                deposit_data(amount),
                ethereum_types::U256::from_dec_str(amount).unwrap(),
            ),
        };
        let gas_price_fast = geth::ethgasstation_fast();
        let nonce = client.nonce(&pub_addr).unwrap();
        let mut token_addr_bytes = [0u8; 20];
        token_addr_bytes.copy_from_slice(&eth::dehex(Weth::CONTRACT_ADDRESS)[..]);
        let tx = ethereum_tx_sign::RawTransaction {
            nonce: ethereum_types::U256::from(nonce),
            to: Some(ethereum_types::H160::from(token_addr_bytes)),
            value: value,
            gas_price: ethereum_types::U256::from(gas_price_fast),
            gas: ethereum_types::U256::from(50000),
            data: data,
        };
        let private_key = ethereum_types::H256::from_slice(&eth::dehex(private_key));
        let rlp_bytes = tx.sign(&private_key, &eth::ETH_CHAIN_MAINNET);
        let params = (eth::hex(&rlp_bytes),);
        let result = client
            .rpc("eth_sendRawTransaction", geth::ParamTypes::Single(params))
            .unwrap();
        match result.part {
            geth::ResultTypes::Error(e) => Err(errors::MainError::build_box(e.error.message)),
            geth::ResultTypes::Result(r) => {
                let tx = r.result;
                println!("GOOD TX {}", tx);
                Ok(true)
            }
        }
    }
}

fn withdraw_data(amount: &str) -> Vec<u8> {
    let mut call = Vec::<u8>::new();
    let mut func = eth::hash_abi_sig("withdraw(uint256)").to_vec();
    call.append(&mut func);
    let mut p1 = hex::decode(eth::encode_uint256(amount)).unwrap();
    call.append(&mut p1);
    call
}

fn deposit_data(amount: &str) -> Vec<u8> {
    let mut call = Vec::<u8>::new();
    let mut func = eth::hash_abi_sig("deposit()").to_vec();
    call.append(&mut func);
    call
}
