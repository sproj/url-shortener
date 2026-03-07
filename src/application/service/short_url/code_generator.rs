use std::collections::VecDeque;

use base64_url::base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use rand::Rng;
use std::sync::Mutex;

pub trait CodeGenerator: Send + Sync {
    fn next_code(&self) -> String;
}

pub struct RandomCodeGenerator;
impl CodeGenerator for RandomCodeGenerator {
    fn next_code(&self) -> String {
        generate_9_bytes_base64url()
    }
}

fn generate_9_bytes_base64url() -> String {
    let mut bytes = vec![0u8; 9];
    rand::rng().fill_bytes(&mut bytes);
    BASE64_URL_SAFE_NO_PAD.encode(&bytes)
}

pub struct FixedCodeGenerator {
    codes: Mutex<VecDeque<String>>,
}
impl CodeGenerator for FixedCodeGenerator {
    fn next_code(&self) -> String {
        self.codes.lock().unwrap().pop_front().unwrap()
    }
}
