pub mod bypass;
pub mod banzhuspider;
pub mod task;
pub mod error;

use crate::error::SpiderError;
use base64::engine::general_purpose;
use base64::Engine;
use cipher::block_padding::Pkcs7;
use cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use opencv::prelude::*;
use pyo3::ffi::c_str;
use pyo3::prelude::{PyAnyMethods, PyModule};
use pyo3::Python;
use rand::rngs::OsRng;
use rand_core::{RngCore, TryRngCore};
use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

pub type Error = Box<dyn std::error::Error + Sync + Send>;

const POST_TEXT: &'static str =
    "&#20026;&#38450;&#27490;&#24694;&#24847;&#35775;&#38382;&#44;&#35831;&#36755;&#20837;【1234】";
const DEFAULT_USER_AGENT: &'static str = "Mozilla/5.0 (Linux; Android 6.0; Nexus 5 Build/MRA58N) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36";

const KEY: &[u8; 16] = b"abcdedghijklmnop"; // 模拟密钥，请勿在实际程序中使用

pub fn get_section_data_by_py(html: &str, ns: &str) -> Result<String> {
    Python::with_gil(|py| {
        let code = c_str!(include_str!("jdom.py"));

        let jdom = PyModule::from_code(py, code, c_str!("jdom.py"), c_str!("jdom")).expect("Unable to load jdom.py");

        let content = jdom.getattr("get_section_data_by_js")?.call1((html, ns))?.extract::<String>().unwrap_or(String::new());
        Ok(content)
    })
}

pub fn decrpyt_aes_128_cbc(cipher_text: &[u8], code: &[u8]) -> Result<Vec<u8>, SpiderError> {
    let m = md5::compute(code);
    let mx = format!("{:x}", m);

    //从code里面拿到key,iv
    let iv = &mx[..16].bytes().collect::<Vec<_>>();
    let key = &mx[16..].bytes().collect::<Vec<_>>();
    // base64解密
    let cipher_text = general_purpose::STANDARD.decode(cipher_text).expect("Error while decoding");

    let cipher_len = cipher_text.len();
    
    let mut buf = vec![0; cipher_len];


    buf[..cipher_len].copy_from_slice(&cipher_text);

    // 解密
    let pt = Aes128CbcDec::new_from_slices(&key, &iv).unwrap()
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .unwrap();
    Ok(pt.to_vec())
}

/// 解密
pub fn decrypt(cipher: &[u8], iv: [u8; 16]) -> Vec<u8> {
    let cipher_len = cipher.len();
    let mut buf = [0u8; 48];
    buf[..cipher_len].copy_from_slice(cipher);

    let pt = Aes128CbcDec::new(KEY.into(), &iv.into())
        .decrypt_padded_b2b_mut::<Pkcs7>(cipher, &mut buf)
        .unwrap();

    pt.to_vec()
}

pub fn encrypt(plain: &[u8]) -> (Vec<u8>, [u8; 16]) {
    let iv = generate_iv();

    let mut buf = [0u8; 48];
    let pt_len = plain.len();
    buf[..pt_len].copy_from_slice(plain);
    let ct = Aes128CbcEnc::new(KEY.into(), &iv.into())
        .encrypt_padded_b2b_mut::<Pkcs7>(plain, &mut buf)
        .unwrap();

    (ct.to_vec(), iv)
}

pub fn get_default_pbr_style() -> ProgressStyle {
    let style = ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len}, {eta})")
        .unwrap()
        .progress_chars("#>-");
    
    style
}

pub fn create_multi_pbr() -> MultiProgress {
    let m = MultiProgress::new();
    m
}

pub fn create_pbr(count: u64) -> ProgressBar {
    let pbr = ProgressBar::new(count);
    
    pbr.set_style(get_default_pbr_style());
    
    pbr
}

/// 生成随机 iv
fn generate_iv() -> [u8; 16] {
    let mut rng = OsRng;
    let mut bytes = [0u8; 16];
    rng.try_fill_bytes(&mut bytes).unwrap();

    bytes
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_aes() {
        let separator = "*".repeat(40);

        let plain = b"This is not a password";
        println!("明文：{:?}", plain);
        let (ct, iv) = encrypt(plain);
        println!(
            "{}\n密文：{:?}\n初始化向量：{:?}\n{}",
            separator, ct, iv, separator
        );
        let pt = decrypt(&ct, iv);
        println!("解密结果：{:?}", pt);

        assert_eq!(plain.to_vec(), pt);
    }
}