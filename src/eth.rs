use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use tiny_keccak::{Hasher, Keccak};

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

#[cfg(test)]
mod tests {
    #[test]
    fn test_pubkey_to_addr() {
        let pubkey_bytes = hex::decode(pubkey).unwrap();
        let mut pubkey_sized_bytes = [0u8; 65];
        pubkey_sized_bytes.copy_from_slice(&pubkey_bytes);
        let addr_bytes = pubkey_to_addr(&pubkey_sized_bytes);
        let addr = hex::encode(addr_bytes);
        assert_eq!(addr, good_addr);
    }
}
