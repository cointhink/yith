use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use tiny_keccak::{Hasher, Keccak};
use std::convert::TryInto;

pub fn wei_to_eth(wei: f64, decimals: i32) -> f64 {
    wei / 10_f64.powi(decimals)
}

//pub fn privkey_to_privkeybytes(privkey: &str) -> [u8; 32] {
//    let secp = Secp256k1::new();
//    let privbytes = &hex::decode(privkey).unwrap();
//    let secret_key = SecretKey::from_slice(privbytes).expect("32 bytes, within curve order");
//}

pub fn privkey_to_pubkeybytes(privkey: &str) -> [u8; 65] {
    let secp = Secp256k1::new();
    let privbytes = &hex::decode(privkey).unwrap();
    let secret_key = SecretKey::from_slice(privbytes).expect("32 bytes, within curve order");
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    public_key.serialize_uncompressed()
}

pub fn privkey_to_addr<'a>(privkey: &str) -> String {
    let pubkey_bytes = privkey_to_pubkeybytes(privkey);
    let addr_bytes = pubkey_to_addr(pubkey_bytes);
    hex::encode(addr_bytes)
}

pub fn pubkey_to_addr(pubkey_bytes: [u8; 65]) -> [u8; 20] {
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(&pubkey_bytes[1..]);
    hasher.finalize(&mut output);
    let mut sized_output = [0u8; 20];
    sized_output.copy_from_slice(&output[12..32]);
    sized_output
}

pub fn ethsign(json: &String, secret_key: &SecretKey) -> String {
    let msg_hash = ethsign_hash_msg(&json.as_bytes().to_vec());
    let sig_bytes = sign_bytes(&msg_hash, &secret_key);
    format!("0x{}", hex::encode(sig_bytes.to_vec()))
}

pub fn ethsign_vrs(json: &Vec<u8>, secret_key: &SecretKey) -> String {
    let msg_hash = ethsign_hash_msg(json);
    let arr = sign_bytes_vrs_arr(&msg_hash, &secret_key);
    format!("0x{}", hex::encode(arr[..].to_vec()))
}

pub fn ethsign_hash_msg(msg: &Vec<u8>) -> [u8; 32] {
    let mut full_msg = format!("\u{0019}Ethereum Signed Message:\n{}", msg.len())
        .as_bytes()
        .to_vec();
    full_msg.append(&mut msg.clone()); // why
    hash_msg(&full_msg)
}

pub fn hash_msg(msg: &Vec<u8>) -> [u8; 32] {
    let mut hash = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(msg);
    hasher.finalize(&mut hash);
    hash
}

pub fn sign_bytes(msg_hash: &[u8], secret_key: &SecretKey) -> [u8; 65] {
    let secp = Secp256k1::new();
    let secp_msg = Message::from_slice(&msg_hash).unwrap();
    let signature = secp.sign_recoverable(&secp_msg, secret_key);
    let (recovery_id, sig) = signature.serialize_compact();
    let mut vec = Vec::with_capacity(65);
    vec.extend_from_slice(&sig);
    // chainId + 27
    let rec_id = recovery_id.to_i32() + 27;
    // rsv form
    vec.push(rec_id as u8);
    let mut sig_sized_bytes = [0u8; 65];
    sig_sized_bytes.copy_from_slice(vec.as_slice());
    sig_sized_bytes
}

pub fn sign_bytes_vrs(msg_hash: &[u8], secret_key: &SecretKey) -> (u8, [u8; 32], [u8; 32]) {
    let secp = Secp256k1::new();
    let secp_msg = Message::from_slice(&msg_hash).unwrap();
    let signature = secp.sign_recoverable(&secp_msg, secret_key);
    let (recovery_id, sig) = signature.serialize_compact();

    // That number between 0 and 3 we call the recovery id, or recid.
    // Therefore, we return an extra byte, which also functions as a header byte,
    // by using 27+recid (for uncompressed recovered pubkeys)
    // or 31+recid (for compressed recovered pubkeys). -- Pieter Wuille
    let v = (recovery_id.to_i32() + 27) as u8;
    let r : [u8; 32] = sig[0..32].try_into().unwrap();
    let s : [u8; 32] = sig[32..64].try_into().unwrap();
    (v,r,s)
}

pub fn sign_bytes_vrs_arr(msg_hash: &[u8], secret_key: &SecretKey) -> [u8; 65] {
    let (v,r,s) = sign_bytes_vrs(&msg_hash, &secret_key);
    let mut sig : [u8; 65] = [0; 65];
    sig[0] = v;
    sig[0..32].copy_from_slice(&r);
    sig[32..64].copy_from_slice(&s);
    sig
}

/*
web3 is giant
pub fn encode(private_key: &str, gas_price: u128, tx: &exchanges::ddex3::OrderTx) -> Vec<u8> {
    const ETH_CHAIN_ID: u32 = 1;

    println!("fullsign relayer {}", tx.relayer);
    let relayer_bytes = hex::decode(tx.relayer[2..].as_bytes()).unwrap();
    let to = Some(web3::types::H160::from_slice(&relayer_bytes));
    let tx = ethereum_tx_sign::RawTransaction {
        nonce: web3::types::U256::from(0),
        to: to,
        value: web3::types::U256::zero(),
        gas_price: web3::types::U256::from(gas_price),
        gas: web3::types::U256::from(tx.gas_token_amount.parse::<u128>().unwrap()),
        data: hex::decode(&tx.data[2..].as_bytes()).unwrap(), //encoded ABI of the contract method 
    };

    /*
    trader:0x9b827e7ee9f127a24eb5243e839007c417c8ac18
    relayer:0x49497a4d914ae91d34ce80030fe620687bf333fd
    baseToken:0x1c95b093d6c236d3ef7c796fe33f9cc6b8606714
    quoteToken:0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2
    baseTokenAmount:5
    quoteTokenAmount:11316000000000000
    gasTokenAmount:1520000000000000
    data:0x02000007b64089450064012c0
    */
    let mut keydata: [u8; 32] = Default::default();
    keydata.copy_from_slice(&hex::decode(private_key).unwrap());
    let private_key = web3::types::H256(keydata);
    let sig_bytes = tx.sign(&private_key, &ETH_CHAIN_ID);
    println!("sig_bytes len {}", sig_bytes.len());
    sig_bytes
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    static privkey: &str = "e4abcbf75d38cf61c4fde0ade1148f90376616f5233b7c1fef2a78c5992a9a50";
    static pubkey: &str = "041ea3510efdb57c6cf0dc77a454b4f5b95775f9606b0f7d7a294b47aae57b21882e6c4888d050992b58a0640066ab72adff7575c07d201716c40b9146624eedb4";
    static good_addr: &str = "ed6d484f5c289ec8c6b6f934ef6419230169f534";
    static msg_v4: &str = "HYDRO-AUTHENTICATION@1566380397473";
    static msg_v3: &str = "HYDRO-AUTHENTICATION@1524088776656";
    static good_sig_v4: &str = "2a10e17a0375a6728947ae4a4ad0fe88e7cc8dd929774be0e33d7e1988f1985f13cf66267134ec4777878b6239e7004b9d2defb03ede94352a20acf0a20a50dc1b";
    static good_sig_v3: &str = "603efd7241bfb6c61f4330facee0f7027d98e030ef241ad03a372638c317859a50620dacee177b771ce05812770a637c4c7395da0042c94250f86fb52472f93500";

    #[test]
    fn test_pubkey_to_addr() {
        let pubkey_bytes = hex::decode(pubkey).unwrap();
        let mut pubkey_sized_bytes = [0u8; 65];
        pubkey_sized_bytes.copy_from_slice(&pubkey_bytes);
        let addr_bytes = pubkey_to_addr(pubkey_sized_bytes);
        let addr = hex::encode(addr_bytes);
        assert_eq!(addr, good_addr);
    }

    #[test]
    fn test_ethsign_hash_msg() {
        //let good_hash_v4 = "68cef504a5bf9b821df3313da9af66354d8865f29ba038c42b62cea53cd9986d";
        let good_hash_v3 = "14d10d289a1662f15e85ddc809acf1f89a888dda71ddaacb1deb60113f6d310f";
        let good_hash_bytes = hex::decode(good_hash_v3).unwrap();
        let mut good_hash_sized_bytes = [0u8; 32];
        good_hash_sized_bytes.copy_from_slice(&good_hash_bytes);
        let hash_bytes = ethsign_hash_msg(&msg_v3.as_bytes().to_vec());
        assert_eq!(hash_bytes, good_hash_sized_bytes);
    }

    #[test]
    fn test_sign_bytes() {
        let hash_v4: &[u8] = b"68cef504a5bf9b821df3313da9af66354d8865f29ba038c42b62cea53cd9986d";
        let hash_v3: &[u8] = b"14d10d289a1662f15e85ddc809acf1f89a888dda71ddaacb1deb60113f6d310f";
        let hash_bytes: Vec<u8> = hex::decode(hash_v4).unwrap();
        let privkey_bytes: Vec<u8> = hex::decode(privkey).unwrap();
        let private_key =
            SecretKey::from_slice(&privkey_bytes).expect("32 bytes, within curve order");
        let sig_bytes = sign_bytes(&hash_bytes, &private_key);
        let good_sig_bytes: Vec<u8> = hex::decode(good_sig_v4).unwrap();
        let mut good_sig_sized_bytes = [0u8; 65];
        good_sig_sized_bytes.copy_from_slice(&good_sig_bytes);
        assert_eq!(&sig_bytes[..], &good_sig_sized_bytes[..]);
    }

    #[test]
    fn test_sign_bytes_vrs() {
        let hash: &[u8] = b"fdc94db5a7aff3bdf03c9dc6188381c6f8fba3ead062c16a6c8b2a59427dd408";
        let hash_bytes: Vec<u8> = hex::decode(hash).unwrap();
        let privkey_bytes: Vec<u8> = hex::decode(privkey).unwrap();
        let private_key = SecretKey::from_slice(&privkey_bytes).unwrap();
        let sig_bytes = sign_bytes_vrs(&hash_bytes, &private_key);
        let good_sig = "1b4ccbff4cb18802ccaf7aaa852595170fc0443d65b1d01a10f5f01d5d65ebe42c58287ecb9cf7f62a98bdfc8931f41a157dd79e9ac5d19880f62089d9c082c79a";
        assert_eq!(hex::encode(&sig_bytes[..]), good_sig);
    }
}
