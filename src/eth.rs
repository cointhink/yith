use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use std::convert::TryInto;
use tiny_keccak::{Hasher, Keccak};

pub const ETH_CHAIN_MAINNET: u32 = 1;

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
    let privbytes = &dehex(privkey);
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
    let (v, r, s) = sign_bytes_vrs(&msg_hash, &secret_key);
    let arr = sigparts_to_rsv(v, r, s);
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
    let (v, r, s) = sign_bytes_vrs(&msg_hash, &secret_key);
    sigparts_to_rsv(v, r, s)
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
    // geth ethapi/api.go:1533
    // signature[64] += 27 // Transform V from 0/1 to 27/28 according to the yellow paper
    let v = (recovery_id.to_i32() + 27) as u8;
    let r: [u8; 32] = sig[0..32].try_into().unwrap();
    let s: [u8; 32] = sig[32..64].try_into().unwrap();
    (v, r, s)
}

pub fn sigparts_to_vrs(v: u8, r: [u8; 32], s: [u8; 32]) -> [u8; 65] {
    let mut sig: [u8; 65] = [0; 65];
    sig[0] = v;
    sig[1..33].copy_from_slice(&r);
    sig[33..65].copy_from_slice(&s);
    sig
}

pub fn sigparts_to_rsv(v: u8, r: [u8; 32], s: [u8; 32]) -> [u8; 65] {
    let mut sig: [u8; 65] = [0; 65];
    sig[0..32].copy_from_slice(&r);
    sig[32..64].copy_from_slice(&s);
    sig[64] = v;
    sig
}

pub fn recover_sig_addr(msg: &[u8], v: u8, r: [u8; 32], s: [u8; 32]) -> [u8; 20] {
    let secp = Secp256k1::new();
    let secp_msg = Message::from_slice(&msg).unwrap();
    let id = secp256k1::recovery::RecoveryId::from_i32(v as i32 - 27).unwrap();
    let mut data = [0u8; 64];
    data[0..32].copy_from_slice(&r);
    data[32..64].copy_from_slice(&s);
    let sig = secp256k1::recovery::RecoverableSignature::from_compact(&data, id).unwrap();
    let pubkey = secp.recover(&secp_msg, &sig).unwrap();
    let pubkey_bytes = pubkey.serialize_uncompressed();
    pubkey_to_addr(pubkey_bytes)
}

pub fn hex(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

pub fn dehex(bytes: &str) -> Vec<u8> {
    hex::decode(bytes.trim_start_matches("0x")).unwrap()
}

pub fn encode_addr2(str: &str) -> Vec<u8> {
    // 160bits/20bytes
    let hexletters = str[2..].to_lowercase();
    let hexletters = hexletters.as_bytes().to_vec();
    left_pad_zero(hexletters, 64)
}

pub fn encode_uint256(numstr: &str) -> Vec<u8> {
    // 256bits/32bytes/64hexchars
    let num = ethereum_types::U256::from_dec_str(numstr).unwrap();
    let number = format!("{:x}", num);
    left_pad_zero(number.as_bytes().to_vec(), 64)
}

pub fn encode_bytes(bytes: &Vec<u8>) -> Vec<u8> {
    let mut buf = Vec::new();
    let bytes_len_str = bytes.len().to_string();
    buf.extend_from_slice(&encode_uint256(0x20.to_string().as_ref()));
    buf.extend_from_slice(&encode_uint256(&bytes_len_str));
    buf.extend_from_slice(&right_pad_zero(hex::encode(bytes).as_bytes().to_vec(), 64));
    buf
}

pub fn left_pad_zero(bytes: Vec<u8>, block_width: usize) -> Vec<u8> {
    let mut padded = Vec::<u8>::new();
    padded.append(&mut pad(&bytes, block_width));
    padded.append(&mut bytes.clone());
    padded
}

pub fn right_pad_zero(bytes: Vec<u8>, block_width: usize) -> Vec<u8> {
    let mut padded = Vec::<u8>::new();
    padded.append(&mut bytes.clone());
    padded.append(&mut pad(&bytes, block_width));
    padded
}

fn pad(bytes: &Vec<u8>, block_width: usize) -> Vec<u8> {
    let padding_char = '0' as u8;
    let mut padded = Vec::<u8>::new();
    let tail_length = bytes.len() % block_width;
    if tail_length > 0 {
        let padding = block_width - tail_length;
        for _ in 0..padding {
            padded.push(padding_char)
        }
        // rustic?
        // padded.extend_from_slice(&iter::repeat(padding_char).take(padding).collect::<Vec<_>>());

    }
    padded
}

pub fn hash_abi_sig(sig: &str) -> [u8; 4] {
    hash_msg(&sig.as_bytes().to_vec())[0..4].try_into().unwrap()
}

pub fn minimum(amounts: &Vec<f64>) -> f64 {
    amounts
        .iter()
        .fold(std::f64::MAX, |memo, f| if *f < memo { *f } else { memo })
}

#[cfg(test)]
mod tests {
    use super::*;
    static PRIVKEY: &str = "e4abcbf75d38cf61c4fde0ade1148f90376616f5233b7c1fef2a78c5992a9a50";
    static PUBKEY: &str = "041ea3510efdb57c6cf0dc77a454b4f5b95775f9606b0f7d7a294b47aae57b21882e6c4888d050992b58a0640066ab72adff7575c07d201716c40b9146624eedb4";
    static GOOD_ADDR: &str = "ed6d484f5c289ec8c6b6f934ef6419230169f534";

    #[test]
    fn test_hash_msg() {
        let bytes = [0u8; 0]; // empty string
        let hash = hash_msg(&bytes.to_vec());
        assert_eq!(
            hex::encode(hash),
            "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470" // old keccak
        );
        // new sha3-256 "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
    }

    #[test]
    fn test_pubkey_to_addr() {
        let pubkey_bytes = hex::decode(PUBKEY).unwrap();
        let mut pubkey_sized_bytes = [0u8; 65];
        pubkey_sized_bytes.copy_from_slice(&pubkey_bytes);
        let addr_bytes = pubkey_to_addr(pubkey_sized_bytes);
        let addr = hex::encode(addr_bytes);
        assert_eq!(addr, GOOD_ADDR);
    }

    #[test]
    fn test_ethsign_hash_msg() {
        //let good_hash_v4 = "68cef504a5bf9b821df3313da9af66354d8865f29ba038c42b62cea53cd9986d";
        let good_hash_v3 = "14d10d289a1662f15e85ddc809acf1f89a888dda71ddaacb1deb60113f6d310f";
        let good_hash_bytes = hex::decode(good_hash_v3).unwrap();
        let mut good_hash_sized_bytes = [0u8; 32];
        good_hash_sized_bytes.copy_from_slice(&good_hash_bytes);
        let msg_v3: &str = "HYDRO-AUTHENTICATION@1524088776656";
        let hash_bytes = ethsign_hash_msg(&msg_v3.as_bytes().to_vec());
        assert_eq!(hash_bytes, good_hash_sized_bytes);
    }

    #[test]
    fn test_sign_bytes() {
        let hash_v4: &[u8] = b"68cef504a5bf9b821df3313da9af66354d8865f29ba038c42b62cea53cd9986d";
        let hash_bytes: Vec<u8> = hex::decode(hash_v4).unwrap();
        let good_sig_v4: &str = "2a10e17a0375a6728947ae4a4ad0fe88e7cc8dd929774be0e33d7e1988f1985f13cf66267134ec4777878b6239e7004b9d2defb03ede94352a20acf0a20a50dc1b";
        let privkey_bytes: Vec<u8> = hex::decode(PRIVKEY).unwrap();
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
        let privkey_bytes: Vec<u8> = hex::decode(PRIVKEY).unwrap();
        let private_key = SecretKey::from_slice(&privkey_bytes).unwrap();
        let (v, r, s) = sign_bytes_vrs(&hash_bytes, &private_key);
        let sig_bytes = sigparts_to_vrs(v, r, s);
        let good_sig = "1b4ccbff4cb18802ccaf7aaa852595170fc0443d65b1d01a10f5f01d5d65ebe42c58287ecb9cf7f62a98bdfc8931f41a157dd79e9ac5d19880f62089d9c082c79a";
        assert_eq!(hex::encode(&sig_bytes[..]), good_sig);
    }

    #[test]
    fn test_recover_sig_addr() {
        let hash: &[u8] = b"fdc94db5a7aff3bdf03c9dc6188381c6f8fba3ead062c16a6c8b2a59427dd408";
        let hash_bytes: Vec<u8> = hex::decode(hash).unwrap();
        let privkey_bytes: Vec<u8> = hex::decode(PRIVKEY).unwrap();
        let private_key = SecretKey::from_slice(&privkey_bytes).unwrap();
        let (v, r, s) = sign_bytes_vrs(&hash_bytes, &private_key);
        let addr = recover_sig_addr(&hash_bytes, v, r, s);
        assert_eq!(hex::encode(&addr), GOOD_ADDR);
    }

    #[test]
    fn test_left_pad_zero() {
        let zero_char = '0' as u8;
        let bytes = vec![1, 2, 3];
        let padded = left_pad_zero(bytes, 4);
        let good = vec![zero_char, 1, 2, 3];
        assert_eq!(good, padded);

        let bytes = vec![1, 2, 3, 4, 5];
        let padded = left_pad_zero(bytes, 4);
        let good = vec![zero_char, zero_char, zero_char, 1, 2, 3, 4, 5];
        assert_eq!(good, padded);

        let bytes = vec![1, 2, 3, 4];
        let padded = left_pad_zero(bytes, 4);
        let good = vec![1, 2, 3, 4];
        assert_eq!(good, padded);
    }

    #[test]
    fn test_encode_uint256() {
        let number = "1";
        let idex_encoded = hex::decode(encode_uint256(number)).unwrap();
        let hash = hash_msg(&idex_encoded);
        let good_hash = "0xb10e2d527612073b26eecdfd717e6a320cf44b4afac2b0732d9fcbe2b7fa0cf6";
        assert_eq!(hex::encode(hash), good_hash[2..]);
        let little_encoded = encode_uint256("1");
        let little_string = String::from_utf8(little_encoded).unwrap();
        let good_little_encoded =
            "0000000000000000000000000000000000000000000000000000000000000001";
        assert_eq!(little_string, good_little_encoded);
        let big_encoded = encode_uint256(
            "115792089237316195423570985008687907853269984665640564039457584007913129639935",
        );
        let big_string = String::from_utf8(big_encoded).unwrap();
        let good_big_encoded = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        assert_eq!(big_string, good_big_encoded);
    }

    #[test]
    fn test_encode_addr2() {
        let address = "0x1122334455667788990011223344556677889900";
        let addr_encoded = encode_addr2(address);
        let addr_str: String = std::str::from_utf8(&addr_encoded).unwrap().to_string();
        let good_hash = "0x0000000000000000000000001122334455667788990011223344556677889900";
        assert_eq!(addr_str, good_hash[2..]);
    }

    #[test]
    fn test_encode_bytes() {
        let mut bytes: Vec<u8> = vec![];
        let out = encode_bytes(&mut bytes);
        assert_eq!(
            std::str::from_utf8(&out).unwrap(),
            "00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000000"
        );

        let mut bytes: Vec<u8> = vec![1, 2, 3];
        let out = encode_bytes(&mut bytes);
        assert_eq!(std::str::from_utf8(&out).unwrap(), 
"000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000030102030000000000000000000000000000000000000000000000000000000000")
    }
}
