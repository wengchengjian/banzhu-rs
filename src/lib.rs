pub mod appconfig;
pub mod crypto;
pub mod db;
pub mod error;
pub mod event;
pub mod scheduler;
pub mod search;
pub mod web;
use crate::error::SpiderError;
use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use cipher::block_padding::Pkcs7;
use cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use rand::rngs::OsRng;
use rand_core::TryRngCore;

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

pub type Error = Box<dyn std::error::Error + Sync + Send>;

pub const DEFAULT_USER_AGENT: &'static str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36";

const KEY: &[u8; 16] = b"abcdedghijklmnop"; // 模拟密钥，请勿在实际程序中使用

pub fn decrpyt_aes_128_cbc(cipher_text: &[u8], code: &[u8]) -> Result<Vec<u8>, SpiderError> {
    let m = md5::compute(code);
    let mx = format!("{:x}", m);

    //从code里面拿到key,iv
    let iv = &mx[..16].bytes().collect::<Vec<_>>();
    let key = &mx[16..].bytes().collect::<Vec<_>>();
    // base64解密
    let cipher_text = general_purpose::STANDARD
        .decode(cipher_text)
        .map_err(|e| SpiderError::DecodingError(format!("base64 decode: {}", e)))?;

    let cipher_len = cipher_text.len();

    let mut buf = vec![0; cipher_len];

    buf[..cipher_len].copy_from_slice(&cipher_text);

    // 解密
    let pt = Aes128CbcDec::new_from_slices(&key, &iv)
        .map_err(|_| SpiderError::DecodingError("invalid AES key/iv length".into()))?
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|_| SpiderError::DecodingError("invalid AES padding".into()))?;
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
