use base64::encode;
use rand;
use rand::Rng;

/// Lenght of the random hash in s3 filename in bytes
static HASH_LEN_BYTES: u8 = 8;

pub trait Random {
    fn generate_hash(&self) -> String;
}

pub struct RandomImpl;

impl RandomImpl {
    pub fn new() -> Self {
        RandomImpl {}
    }

    /// Three symbols +, /, = are not aws and url-friendly, just replace them
    fn encode_for_aws(s: &str) -> String {
        let s = s.replace("+", "A");
        let s = s.replace("/", "B");
        s.replace("=", "C")
    }
}

impl Random for RandomImpl {
    fn generate_hash(&self) -> String {
        let mut name_bytes = vec![0; HASH_LEN_BYTES as usize];
        let buffer = name_bytes.as_mut_slice();
        rand::thread_rng().fill_bytes(buffer);
        Self::encode_for_aws(&encode(buffer))
    }
}
