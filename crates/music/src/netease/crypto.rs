//! The request encryption NetEase's endpoints expect, ported from Mineral's NetEase channel
//! (MIT, © 2025 wanger, <https://github.com/10knamesmore/Mineral>).
//!
//! Two schemes are spoken. `weapi` is what the web player sends: the JSON is AES-CBC encrypted
//! twice, the second time under a random key that travels RSA-wrapped beside it. `linuxapi` is
//! the old Linux client's: the whole request, method and url included, AES-ECB encrypted under
//! one fixed key and forwarded by the server. Both answer a form body ready to POST.

use aes::Aes128;
use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockEncrypt as _, BlockEncryptMut as _, KeyInit as _, KeyIvInit as _};
use base64::Engine as _;
use num_bigint::BigUint;

use crate::escape;

/// The first AES-CBC key of `weapi`.
const PRESET_KEY: &[u8; 16] = b"0CoJUm6Qyw8W8jud";
/// The IV both `weapi` layers use.
const IV: &[u8; 16] = b"0102030405060708";
/// The AES-ECB key of `linuxapi`. Its `&`, `#` and `?` are key bytes, never url syntax.
const LINUX_KEY: &[u8; 16] = b"rFgB&h#%2?^eDg:Q";
/// What the random `weapi` key is drawn from.
const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
/// The modulus of the 1024-bit RSA key the web player wraps its random key with, the same key as
/// the PEM NetEase ships, written out so nothing has to parse ASN.1 at run time.
const MODULUS: &str = "e0b509f6259df8642dbc35662901477df22677ec152b5ff68ace615bb7b725152b3ab17a876aea8a5aa76d2e417629ec4ee341f56135fccf695280104e0312ecbda92557c93870114af6c9d05c4f7f0c3685b7a46bee255932575cce10b424d813cfe4875d3e82047b97ddef52741d546b8e289dc6935b3ece0462db0a22b8e7";
const EXPONENT: u32 = 65_537;

/// The `params=…&encSecKey=…` body of a `weapi` call carrying `json`.
pub(crate) fn weapi(json: &str) -> String {
    let mut key = [0u8; 16];
    for byte in &mut key {
        *byte = ALPHABET[fastrand::usize(..ALPHABET.len())];
    }
    weapi_with(json, &key)
}

/// `weapi` under a chosen random key, so a test can pin the output. The second AES layer takes
/// the key reversed, while the RSA layer wraps it as drawn, which is what the server undoes.
fn weapi_with(json: &str, key: &[u8; 16]) -> String {
    let base64 = base64::engine::general_purpose::STANDARD;
    let inner = base64.encode(cbc(json.as_bytes(), PRESET_KEY));
    let mut reversed = *key;
    reversed.reverse();
    let params = base64.encode(cbc(inner.as_bytes(), &reversed));
    let wrapped = hex(&rsa(key), false);
    format!(
        "params={}&encSecKey={}",
        escape::component(&params),
        escape::component(&wrapped)
    )
}

/// The `eparams=…` body of a `linuxapi` call carrying `json`, the whole forwarded request.
pub(crate) fn linuxapi(json: &str) -> String {
    format!(
        "eparams={}",
        escape::component(&hex(&ecb(json.as_bytes(), LINUX_KEY), true))
    )
}

/// AES-128-CBC with PKCS#7 padding under the fixed IV.
fn cbc(plain: &[u8], key: &[u8; 16]) -> Vec<u8> {
    let mut out = plain.to_vec();
    out.resize(plain.len() + 16 - plain.len() % 16, 0);
    let encryptor = cbc::Encryptor::<Aes128>::new(key.into(), IV.into());
    // The buffer was grown to the padded length above, so padding always has room.
    if encryptor
        .encrypt_padded_mut::<Pkcs7>(&mut out, plain.len())
        .is_err()
    {
        out.clear();
    }
    out
}

/// AES-128-ECB with PKCS#7 padding: every 16-byte block encrypted on its own.
fn ecb(plain: &[u8], key: &[u8; 16]) -> Vec<u8> {
    let cipher = Aes128::new(key.into());
    let fill = 16 - plain.len() % 16;
    let mut out = plain.to_vec();
    out.resize(plain.len() + fill, fill as u8);
    for block in out.as_chunks_mut::<16>().0 {
        cipher.encrypt_block(block.into());
    }
    out
}

/// Textbook RSA with no padding: the key left-filled with zeros to 128 bytes, raised to the
/// public exponent, and the result left-filled back to 128 bytes.
fn rsa(key: &[u8; 16]) -> Vec<u8> {
    let modulus = BigUint::parse_bytes(MODULUS.as_bytes(), 16).unwrap_or_default();
    let message = BigUint::from_bytes_be(key);
    let sealed = message
        .modpow(&BigUint::from(EXPONENT), &modulus)
        .to_bytes_be();
    let mut out = vec![0u8; 128usize.saturating_sub(sealed.len())];
    out.extend(sealed);
    out
}

/// Bytes as hex digits, in upper case when `upper` is set.
fn hex(bytes: &[u8], upper: bool) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        match upper {
            true => write!(out, "{byte:02X}"),
            false => write!(out, "{byte:02x}"),
        }
        .ok();
        out
    })
}

#[cfg(test)]
mod tests {
    //! The expected values were computed apart from this code, with Python's `cryptography`
    //! package for AES and Python's own big integers for RSA.

    use super::*;

    #[test]
    fn cbc_matches_the_reference() {
        let cases: [(&[u8], &str); 3] = [
            (b"", "d714c95766463ee3c00d0667b559f0cc"),
            (b"hello", "f89f50def2f32c616eaa5585421dd3dc"),
            (
                br#"{"id":"123","csrf_token":""}"#,
                "7ed73c5ac753b09a4b726b6395af2c406cc16e22c0a43369477286bcd81b2af3",
            ),
        ];
        for (plain, expected) in cases {
            assert_eq!(hex(&cbc(plain, PRESET_KEY), false), expected);
        }
    }

    #[test]
    fn ecb_matches_the_reference() {
        assert_eq!(
            hex(&ecb(b"", LINUX_KEY), false),
            "e254137cd4dcfef7e51a9955d586b94f"
        );
        assert_eq!(
            hex(&ecb(b"hello", LINUX_KEY), false),
            "7c9d21fdfe1c9186ecab2a729f89a42d"
        );
    }

    #[test]
    fn rsa_matches_the_reference() {
        assert_eq!(
            hex(&rsa(b"AAAAAAAAAAAAAAAA"), false),
            "093601d1eae1e2c4478ba94b7f407da4b5feb00ff5e5383e4710bceac5f567739a5708c19e13f4d286a106720ec6d8af850b90d442aa623810a2c42fe43a66a879bde7031b99ab059bd28ad193501406923db7748b59468a13402bf65a0dedbf36351d15175067d722db5bb386b7dd079c1d76ddd194cb342d307705264608dd"
        );
        let sealed = rsa(b"q9Z3kPxL2VmH8nWb");
        assert_eq!(sealed.len(), 128);
        assert_eq!(
            hex(&sealed, false),
            "641a60c3c6b22d02374b28f778aba186fc6c483ac10ec0bf8fe60635bffffefedae6fcf6097dca7d6b94031f6c7603e1d3f528904e56556e2947e746a2e4b0541895b0b2f7307cda43b0017bd79e731b228d04fc41da624e64b48512f602666be3c32f3714876e7bbbeb327b0e25050f9c51ff02b44619504a42f854e6c525fa"
        );
    }

    #[test]
    fn weapi_body_matches_the_reference() {
        let json = r#"{"s":"hello","type":"1","offset":"0","limit":"30","csrf_token":""}"#;
        assert_eq!(
            weapi_with(json, b"0123456789abcdef"),
            "params=jhTRa5vPTOO6buQCsrOFxtp%2Fl7ppmbrpABEdbDWW7egZwVuU9TYcoSV2jw174t%2FivIRz9DbeRo6iQy3hQ1nV%2FIHtQUw%2FM6O3a28zLdxnjJAyWKgo1yu%2FLdRHuN1ObbS7Rrt4PzXFyYFiFdy5I8M%2FLQ%3D%3D&encSecKey=ac744caab466f1fd5a75228365d15df7add288e8982a2c8f80da5f8f22d54834b5e1eab04dde9dac7341851a98de37463d8ba01d9e9c4d2c546c0f948e3163c667785a1b9c4f160305b0fb1ec3b698de0c704719b2fbc469582654b9c2595317dc40c3ecee32baa6d9753970b66667c01bde9ad34048fa8b2b0f628e56a5bf49"
        );
    }

    #[test]
    fn linuxapi_body_matches_the_reference() {
        let json = r#"{"method":"POST","url":"https://music.163.com/api/song/detail","params":{"ids":"[123]"}}"#;
        assert_eq!(
            linuxapi(json),
            "eparams=A0D9583F4C5FF68DE851D2893A49DE98FAFB24399F27B4F7E74C64B6FC49A965CFA972FA5EA3D6247CD6247C8198CB8724902E133AF06ABB1602F129E968462DFB0362040EC3189DE4690C55FA6E206D0CF5F7172F8528C459334BD08EFBAF39"
        );
    }

    #[test]
    fn the_random_key_comes_from_the_alphabet() {
        let body = weapi("{}");
        assert!(body.starts_with("params="));
        assert!(body.contains("&encSecKey="));
        let wrapped = body.split("&encSecKey=").nth(1).unwrap_or_default();
        assert_eq!(wrapped.len(), 256);
    }
}
