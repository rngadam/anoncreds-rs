#[cfg(feature = "ed25519")]
use ursa::signatures::{ed25519::Ed25519Sha512, SignatureScheme};

use zeroize::Zeroize;

use super::base58;
use super::error::ConversionError;
use super::{Validatable, ValidationError};

mod types;
pub use types::*;

#[cfg(feature = "ed25519")]
lazy_static! {
    pub static ref ED25519_SIGNER: Ed25519Sha512 = Ed25519Sha512::new();
}

pub fn build_full_verkey(dest: &str, key: &str) -> Result<EncodedVerKey, ConversionError> {
    EncodedVerKey::from_str_qualified(key, Some(dest), None, None)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignKey {
    pub key: Vec<u8>,
    pub alg: KeyType,
}

impl SignKey {
    pub fn new<K: AsRef<[u8]>>(key: K, alg: Option<KeyType>) -> Self {
        Self {
            key: key.as_ref().to_vec(),
            alg: alg.unwrap_or_default(),
        }
    }

    #[cfg(feature = "ed25519")]
    pub fn generate(alg: Option<KeyType>) -> Result<Self, ConversionError> {
        let alg = alg.unwrap_or_default();
        match alg {
            KeyType::ED25519 => {
                let (_pk, sk) = ED25519_SIGNER
                    .keypair(None)
                    .map_err(|_| "Error creating signing key")?;
                Ok(Self::new(sk, Some(KeyType::ED25519)))
            }
            _ => Err("Unsupported key type".into()),
        }
    }

    #[cfg(feature = "ed25519")]
    pub fn from_seed(seed: &[u8]) -> Result<Self, ConversionError> {
        let (_pk, sk) = Ed25519Sha512::expand_keypair(seed)
            .map_err(|err| format!("Error creating signing key: {}", err))?;
        Ok(Self::new(sk, Some(KeyType::ED25519)))
    }

    pub fn public_key(&self) -> Result<VerKey, ConversionError> {
        match self.alg {
            KeyType::ED25519 => Ok(VerKey::new(&self.key[32..], Some(self.alg.clone()))),
            _ => Err("Unsupported key type".into()),
        }
    }

    pub fn key_bytes(&self) -> Vec<u8> {
        self.key.clone()
    }

    #[cfg(feature = "ed25519")]
    pub fn key_exchange(&self) -> Result<Self, ConversionError> {
        match self.alg {
            KeyType::ED25519 => {
                let sk = ursa::keys::PrivateKey(self.key_bytes());
                let x_sk = Ed25519Sha512::sign_key_to_key_exchange(&sk)
                    .map_err(|err| format!("Error converting to x25519 key: {}", err))?;
                Ok(Self::new(&x_sk, Some(KeyType::X25519)))
            }
            _ => Err("Unsupported key format for key exchange".into()),
        }
    }

    #[cfg(feature = "ed25519")]
    pub fn sign<M: AsRef<[u8]>>(&self, message: M) -> Result<Vec<u8>, ConversionError> {
        match self.alg {
            KeyType::ED25519 => {
                let sk = ursa::keys::PrivateKey(self.key_bytes());
                Ok(ED25519_SIGNER
                    .sign(message.as_ref(), &sk)
                    .map_err(|err| format!("Error signing payload: {}", err))?)
            }
            _ => Err("Unsupported key format for signing".into()),
        }
    }
}

impl AsRef<[u8]> for SignKey {
    fn as_ref(&self) -> &[u8] {
        self.key.as_ref()
    }
}

impl Zeroize for SignKey {
    fn zeroize(&mut self) {
        self.key.zeroize();
        self.alg = KeyType::from("")
    }
}

impl Drop for SignKey {
    fn drop(&mut self) {
        self.zeroize()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerKey {
    pub key: Vec<u8>,
    pub alg: KeyType,
}

impl VerKey {
    pub fn new<K: AsRef<[u8]>>(key: K, alg: Option<KeyType>) -> Self {
        let alg = alg.unwrap_or_default();
        Self {
            key: key.as_ref().to_vec(),
            alg,
        }
    }

    pub fn as_base58(&self) -> Result<EncodedVerKey, ConversionError> {
        self.encode(KeyEncoding::BASE58)
    }

    pub fn encode(&self, enc: KeyEncoding) -> Result<EncodedVerKey, ConversionError> {
        match enc {
            enc @ KeyEncoding::BASE58 => {
                let key = base58::encode(&self.key);
                Ok(EncodedVerKey::new(
                    key.as_str(),
                    Some(self.alg.clone()),
                    Some(enc),
                ))
            }
            _ => Err("Unsupported key encoding".into()),
        }
    }

    pub fn key_bytes(&self) -> Vec<u8> {
        self.key.clone()
    }

    #[cfg(feature = "ed25519")]
    pub fn key_exchange(&self) -> Result<Self, ConversionError> {
        match self.alg {
            KeyType::ED25519 => {
                let vk = ursa::keys::PublicKey(self.key_bytes());
                let x_vk = Ed25519Sha512::ver_key_to_key_exchange(&vk).map_err(|err| {
                    format!("Error converting to x25519 key: {}", err.to_string())
                })?;
                Ok(Self::new(&x_vk, Some(KeyType::X25519)))
            }
            _ => Err("Unsupported verkey type".into()),
        }
    }

    #[cfg(feature = "ed25519")]
    pub fn verify_signature<M: AsRef<[u8]>, S: AsRef<[u8]>>(
        &self,
        message: M,
        signature: S,
    ) -> Result<bool, ConversionError> {
        match self.alg {
            KeyType::ED25519 => {
                let vk = ursa::keys::PublicKey(self.key_bytes());
                Ok(ED25519_SIGNER
                    .verify(message.as_ref(), signature.as_ref(), &vk)
                    .map_err(|err| format!("Error validating message signature: {}", err))?)
            }
            _ => Err("Unsupported verkey type".into()),
        }
    }
}

impl AsRef<[u8]> for VerKey {
    fn as_ref(&self) -> &[u8] {
        self.key.as_ref()
    }
}

impl std::fmt::Display for VerKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.as_base58() {
            Ok(k) => k.fmt(f),
            Err(err) => write!(f, "<Error encoding key: {}>", err),
        }
    }
}

impl Validatable for VerKey {
    fn validate(&self) -> Result<(), ValidationError> {
        let bytes = self.key_bytes();
        if bytes.len() == 32 {
            Ok(())
        } else {
            Err("Invalid key length".into())
        }
    }
}

impl Zeroize for VerKey {
    fn zeroize(&mut self) {
        self.key.zeroize();
        self.alg = KeyType::from("");
    }
}

impl Drop for VerKey {
    fn drop(&mut self) {
        self.zeroize()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedVerKey {
    pub key: String,
    pub alg: KeyType,
    pub enc: KeyEncoding,
}

impl EncodedVerKey {
    pub fn new(key: &str, alg: Option<KeyType>, enc: Option<KeyEncoding>) -> Self {
        let alg = alg.unwrap_or_default();
        let enc = enc.unwrap_or_default();
        Self {
            key: key.to_owned(),
            alg,
            enc,
        }
    }

    pub fn from_slice<K: AsRef<[u8]>>(key: K) -> Result<Self, ConversionError> {
        let key = std::str::from_utf8(key.as_ref())?;
        Self::from_str_qualified(key, None, None, None)
    }

    pub fn from_str(key: &str) -> Result<Self, ConversionError> {
        Self::from_str_qualified(key, None, None, None)
    }

    pub fn from_str_qualified(
        key: &str,
        dest: Option<&str>,
        alg: Option<KeyType>,
        enc: Option<KeyEncoding>,
    ) -> Result<Self, ConversionError> {
        let (key, alg) = if key.contains(':') {
            let splits: Vec<&str> = key.splitn(2, ':').collect();
            let alg = match splits[1] {
                "" => alg,
                _ => Some(splits[1].into()),
            };
            (splits[0], alg)
        } else {
            (key, alg)
        };

        if key.starts_with('~') {
            let dest =
                unwrap_opt_or_return!(dest, Err("Destination required for short verkey".into()));
            let mut result = base58::decode(dest)?;
            let mut end = base58::decode(&key[1..])?;
            result.append(&mut end);
            Ok(Self::new(&base58::encode(result), alg, enc))
        } else {
            Ok(Self::new(key, alg, enc))
        }
    }

    pub fn long_form(&self) -> String {
        let mut result = self.key.clone();
        result.push(':');
        result.push_str(&self.alg);
        result
    }

    pub fn as_base58(self) -> Result<Self, ConversionError> {
        match self.enc {
            KeyEncoding::BASE58 => Ok(self),
            _ => {
                let key = base58::encode(self.key_bytes()?);
                Ok(Self::new(
                    key.as_str(),
                    Some(self.alg.clone()),
                    Some(KeyEncoding::BASE58),
                ))
            }
        }
    }

    pub fn key_bytes(&self) -> Result<Vec<u8>, ConversionError> {
        match self.enc {
            KeyEncoding::BASE58 => Ok(base58::decode(&self.key)?),
            _ => Err("Unsupported verkey format".into()),
        }
    }

    pub fn encoded_key_bytes(&self) -> &[u8] {
        self.key.as_bytes()
    }

    #[cfg(feature = "ed25519")]
    pub fn key_exchange(&self) -> Result<ursa::keys::PublicKey, ConversionError> {
        match self.alg {
            KeyType::ED25519 => {
                let vk = ursa::keys::PublicKey(self.key_bytes()?);
                Ok(Ed25519Sha512::ver_key_to_key_exchange(&vk).map_err(|err| {
                    format!("Error converting to x25519 key: {}", err.to_string())
                })?)
            }
            _ => Err("Unsupported verkey type".into()),
        }
    }

    #[cfg(feature = "ed25519")]
    pub fn verify_signature<M: AsRef<[u8]>, S: AsRef<[u8]>>(
        &self,
        message: M,
        signature: S,
    ) -> Result<bool, ConversionError> {
        match self.alg {
            KeyType::ED25519 => {
                let vk = ursa::keys::PublicKey(self.key_bytes()?);
                Ok(ED25519_SIGNER
                    .verify(message.as_ref(), signature.as_ref(), &vk)
                    .map_err(|err| format!("Error validating message signature: {}", err))?)
            }
            _ => Err("Unsupported verkey type".into()),
        }
    }
}

impl std::fmt::Display for EncodedVerKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = if self.alg == KeyType::default() {
            self.key.clone()
        } else {
            self.long_form()
        };
        f.write_str(out.as_str())
    }
}

impl Validatable for EncodedVerKey {
    fn validate(&self) -> Result<(), ValidationError> {
        let bytes = self.key_bytes()?;
        if bytes.len() == 32 {
            Ok(())
        } else {
            Err("Invalid key length".into())
        }
    }
}

impl Zeroize for EncodedVerKey {
    fn zeroize(&mut self) {
        self.key.zeroize();
        self.alg = KeyType::from("");
        self.enc = KeyEncoding::from("")
    }
}

impl Drop for EncodedVerKey {
    fn drop(&mut self) {
        self.zeroize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_empty() {
        assert_eq!(
            EncodedVerKey::from_str("").unwrap(),
            EncodedVerKey::new("", Some(KeyType::default()), Some(KeyEncoding::default()))
        )
    }

    #[test]
    fn from_str_single_colon() {
        assert_eq!(
            EncodedVerKey::from_str(":").unwrap(),
            EncodedVerKey::new("", Some(KeyType::default()), Some(KeyEncoding::default()))
        )
    }

    #[test]
    fn from_str_ends_with_colon() {
        assert_eq!(
            EncodedVerKey::from_str("foo:").unwrap(),
            EncodedVerKey::new(
                "foo",
                Some(KeyType::default()),
                Some(KeyEncoding::default())
            )
        )
    }

    #[test]
    fn from_key_starts_with_colon() {
        assert_eq!(
            EncodedVerKey::from_str(":bar").unwrap(),
            EncodedVerKey::new("", Some("bar".into()), Some(KeyEncoding::default()))
        )
    }

    #[test]
    fn from_key_works() {
        assert_eq!(
            EncodedVerKey::from_str("foo:bar:baz").unwrap(),
            EncodedVerKey::new("foo", Some("bar:baz".into()), Some(KeyEncoding::default()))
        )
    }

    #[test]
    fn round_trip_verkey() {
        assert_eq!(
            EncodedVerKey::from_str("foo:bar:baz").unwrap().long_form(),
            "foo:bar:baz"
        )
    }

    #[cfg(feature = "ed25519")]
    #[test]
    fn sign_and_verify() {
        let message = b"hello there";
        let sk = SignKey::generate(None).unwrap();
        let sig = sk.sign(&message).unwrap();
        let vk = sk.public_key().unwrap();
        assert!(vk.verify_signature(&message, &sig).unwrap());
    }
}
