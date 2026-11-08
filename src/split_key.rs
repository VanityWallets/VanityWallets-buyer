//! Разделённый ключ (split-key) для vanity-адресов на кривой secp256k1:
//! TRON, ETH и EVM-сети, BTC, LTC, DOGE, DASH, XRP и Cosmos-сети. Общий код
//! программы покупателя и программы продавца
//! (продавец подключает этот файл к себе), поэтому форматы кодов и вывод
//! адресов у них гарантированно одинаковые.
//!
//! Схема:
//! 1. Покупатель создаёт секрет `a`, оставляет его у себя и отправляет
//!    продавцу публичную половину `A = a·G` (код заявки `VGREQ1`).
//! 2. Продавец перебирает свои числа `b`, пока адрес точки `A + b·G` не
//!    подойдёт под шаблон. Сам кошелёк продавцу недоступен: для него нужен
//!    ключ `a + b`, а `a` есть только у покупателя.
//! 3. До оплаты продавец отправляет адрес и `B = b·G` (код `VGPROOF1`):
//!    покупатель убеждается, что адрес собран из его половины.
//! 4. После оплаты продавец отправляет `b` (код `VGKEY1`), покупатель
//!    вычисляет полный ключ `k = a + b mod n` и проверяет адрес.

use bech32::{hrp, segwit, Bech32, Hrp};
use secp256k1::{PublicKey, Scalar, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};
use std::fmt;
use std::sync::OnceLock;

const REQ_TAG: &str = "VGREQ1";
const PROOF_TAG: &str = "VGPROOF1";
const KEY_TAG: &str = "VGKEY1";

fn secp() -> &'static Secp256k1<secp256k1::All> {
    static CTX: OnceLock<Secp256k1<secp256k1::All>> = OnceLock::new();
    CTX.get_or_init(Secp256k1::new)
}

/// Сеть и тип адреса. Метки первых шести сетей не менялись с первой версии:
/// старые заявки и коды остаются действительными.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Network {
    Tron,
    Eth,
    BtcP2pkh,
    BtcP2sh,
    BtcP2wpkh,
    BtcP2tr,
    // EVM-сети: адрес и ключ те же, что у Ethereum.
    Bsc,
    Polygon,
    Arbitrum,
    Optimism,
    Base,
    Avalanche,
    Linea,
    ZkSync,
    HyperEvm,
    LtcP2pkh,
    LtcP2sh,
    LtcP2wpkh,
    Doge,
    Dash,
    Xrp,
    // Cosmos-сети: HASH160 сжатого ключа в bech32 со своим префиксом.
    Cosmos,
    Osmosis,
    Celestia,
    Dydx,
    Akash,
    Juno,
    Stargaze,
    Noble,
}

/// Как из публичного ключа получается адрес сети.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// 0x41 || последние 20 байт Keccak-256 → Base58Check.
    Tron,
    /// Последние 20 байт Keccak-256 → hex EIP-55.
    Evm,
    /// версия || HASH160(сжатый ключ) → Base58Check; `p2sh` — HASH160 скрипта
    /// 0x0014 || HASH160 (P2SH-P2WPKH); `ripple` — алфавит XRP Ledger.
    Base58 { version: u8, p2sh: bool, ripple: bool },
    /// SegWit v0: HASH160(сжатый ключ) → bech32 с префиксом `hrp`.
    SegwitV0 { hrp: &'static str },
    /// Bitcoin Taproot (BIP86) → bech32m.
    Taproot,
    /// Cosmos: HASH160(сжатый ключ) → bech32 без версии свидетеля.
    Bech32 { hrp: &'static str },
}

/// Алфавит Base58 XRP Ledger (та же нумерация цифр, другие символы).
/// Нужен программе продавца (перевод шаблонов в цифры для GPU).
#[allow(dead_code)]
pub const RIPPLE_ALPHABET: &[u8; 58] = b"rpshnaf39wBUDNEGHJKLM4PQRST7VWXYZ2bcdeCg65jkm8oFqi1tuvAxyz";

impl Network {
    pub const ALL: [Network; 29] = [
        Network::Tron,
        Network::Eth,
        Network::Bsc,
        Network::Polygon,
        Network::Arbitrum,
        Network::Optimism,
        Network::Base,
        Network::Avalanche,
        Network::Linea,
        Network::ZkSync,
        Network::HyperEvm,
        Network::BtcP2pkh,
        Network::BtcP2sh,
        Network::BtcP2wpkh,
        Network::BtcP2tr,
        Network::LtcP2pkh,
        Network::LtcP2sh,
        Network::LtcP2wpkh,
        Network::Doge,
        Network::Dash,
        Network::Xrp,
        Network::Cosmos,
        Network::Osmosis,
        Network::Celestia,
        Network::Dydx,
        Network::Akash,
        Network::Juno,
        Network::Stargaze,
        Network::Noble,
    ];

    /// Метка сети внутри кодов.
    pub fn tag(self) -> &'static str {
        match self {
            Network::Tron => "TRON",
            Network::Eth => "ETH",
            Network::BtcP2pkh => "BTC-P2PKH",
            Network::BtcP2sh => "BTC-P2SH",
            Network::BtcP2wpkh => "BTC-P2WPKH",
            Network::BtcP2tr => "BTC-P2TR",
            Network::Bsc => "BSC",
            Network::Polygon => "POLYGON",
            Network::Arbitrum => "ARBITRUM",
            Network::Optimism => "OPTIMISM",
            Network::Base => "BASE",
            Network::Avalanche => "AVAX-C",
            Network::Linea => "LINEA",
            Network::ZkSync => "ZKSYNC",
            Network::HyperEvm => "HYPEREVM",
            Network::LtcP2pkh => "LTC-P2PKH",
            Network::LtcP2sh => "LTC-P2SH",
            Network::LtcP2wpkh => "LTC-P2WPKH",
            Network::Doge => "DOGE",
            Network::Dash => "DASH",
            Network::Xrp => "XRP",
            Network::Cosmos => "COSMOS",
            Network::Osmosis => "OSMOSIS",
            Network::Celestia => "CELESTIA",
            Network::Dydx => "DYDX",
            Network::Akash => "AKASH",
            Network::Juno => "JUNO",
            Network::Stargaze => "STARGAZE",
            Network::Noble => "NOBLE",
        }
    }

    pub fn from_tag(tag: &str) -> Option<Network> {
        Network::ALL.into_iter().find(|n| n.tag() == tag)
    }

    /// Название для людей.
    pub fn label(self) -> &'static str {
        match self {
            Network::Tron => "TRON (TRX / TRC20)",
            Network::Eth => "Ethereum (ETH / ERC20)",
            Network::BtcP2pkh => "Bitcoin Legacy (1…)",
            Network::BtcP2sh => "Bitcoin P2SH-SegWit (3…)",
            Network::BtcP2wpkh => "Bitcoin Native SegWit (bc1q…)",
            Network::BtcP2tr => "Bitcoin Taproot (bc1p…)",
            Network::Bsc => "BNB Smart Chain (BSC / BEP20)",
            Network::Polygon => "Polygon (POL)",
            Network::Arbitrum => "Arbitrum One",
            Network::Optimism => "Optimism (OP Mainnet)",
            Network::Base => "Base",
            Network::Avalanche => "Avalanche C-Chain",
            Network::Linea => "Linea",
            Network::ZkSync => "zkSync Era",
            Network::HyperEvm => "HyperEVM (HYPE)",
            Network::LtcP2pkh => "Litecoin Legacy (L…)",
            Network::LtcP2sh => "Litecoin P2SH-SegWit (M…)",
            Network::LtcP2wpkh => "Litecoin Native SegWit (ltc1q…)",
            Network::Doge => "Dogecoin (D…)",
            Network::Dash => "Dash (X…)",
            Network::Xrp => "XRP Ledger (r…)",
            Network::Cosmos => "Cosmos Hub (cosmos1…)",
            Network::Osmosis => "Osmosis (osmo1…)",
            Network::Celestia => "Celestia (celestia1…)",
            Network::Dydx => "dYdX (dydx1…)",
            Network::Akash => "Akash (akash1…)",
            Network::Juno => "Juno (juno1…)",
            Network::Stargaze => "Stargaze (stars1…)",
            Network::Noble => "Noble (noble1…)",
        }
    }

    /// Вывод адреса сети.
    pub fn encoding(self) -> Encoding {
        let b58 = |version, p2sh| Encoding::Base58 { version, p2sh, ripple: false };
        match self {
            Network::Tron => Encoding::Tron,
            Network::Eth
            | Network::Bsc
            | Network::Polygon
            | Network::Arbitrum
            | Network::Optimism
            | Network::Base
            | Network::Avalanche
            | Network::Linea
            | Network::ZkSync
            | Network::HyperEvm => Encoding::Evm,
            Network::BtcP2pkh => b58(0x00, false),
            Network::BtcP2sh => b58(0x05, true),
            Network::BtcP2wpkh => Encoding::SegwitV0 { hrp: "bc" },
            Network::BtcP2tr => Encoding::Taproot,
            Network::LtcP2pkh => b58(0x30, false),
            Network::LtcP2sh => b58(0x32, true),
            Network::LtcP2wpkh => Encoding::SegwitV0 { hrp: "ltc" },
            Network::Doge => b58(0x1e, false),
            Network::Dash => b58(0x4c, false),
            Network::Xrp => Encoding::Base58 { version: 0x00, p2sh: false, ripple: true },
            Network::Cosmos => Encoding::Bech32 { hrp: "cosmos" },
            Network::Osmosis => Encoding::Bech32 { hrp: "osmo" },
            Network::Celestia => Encoding::Bech32 { hrp: "celestia" },
            Network::Dydx => Encoding::Bech32 { hrp: "dydx" },
            Network::Akash => Encoding::Bech32 { hrp: "akash" },
            Network::Juno => Encoding::Bech32 { hrp: "juno" },
            Network::Stargaze => Encoding::Bech32 { hrp: "stars" },
            Network::Noble => Encoding::Bech32 { hrp: "noble" },
        }
    }

    /// EVM-сеть: адрес «0x…» как у Ethereum.
    pub fn is_evm(self) -> bool {
        self.encoding() == Encoding::Evm
    }

    /// Байт версии WIF для сетей, кошельки которых импортируют ключ в WIF.
    pub fn wif_version(self) -> Option<u8> {
        match self {
            Network::BtcP2pkh | Network::BtcP2sh | Network::BtcP2wpkh | Network::BtcP2tr => Some(0x80),
            Network::LtcP2pkh | Network::LtcP2sh | Network::LtcP2wpkh => Some(0xb0),
            Network::Doge => Some(0x9e),
            Network::Dash => Some(0xcc),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitError {
    /// Строка не похожа на код нужного вида.
    BadFormat,
    /// Контрольная сумма не сошлась: код скопирован с ошибкой.
    BadChecksum,
    /// Ключ или точка вне кривой.
    BadKey,
    UnknownNetwork,
    /// Код относится к другой заявке.
    WrongOrder,
    /// Адрес не получается из этих половин.
    AddressMismatch,
    /// Половина продавца не совпадает с ранее проверенной.
    ProofMismatch,
}

impl SplitError {
    pub fn text(self, ru: bool) -> &'static str {
        match (self, ru) {
            (SplitError::BadFormat, true) => "Это не код нужного вида. Скопируйте его целиком.",
            (SplitError::BadFormat, false) => "This is not a code of the expected kind. Copy it in full.",
            (SplitError::BadChecksum, true) => "Код повреждён (не сошлась контрольная сумма). Скопируйте его заново.",
            (SplitError::BadChecksum, false) => "The code is damaged (checksum mismatch). Copy it again.",
            (SplitError::BadKey, true) => "В коде недопустимый ключ.",
            (SplitError::BadKey, false) => "The code contains an invalid key.",
            (SplitError::UnknownNetwork, true) => "В коде неизвестная сеть.",
            (SplitError::UnknownNetwork, false) => "The code names an unknown network.",
            (SplitError::WrongOrder, true) => "Код относится к другой заявке.",
            (SplitError::WrongOrder, false) => "The code belongs to another request.",
            (SplitError::AddressMismatch, true) => {
                "Адрес НЕ получается из вашей половины ключа. Не используйте этот адрес!"
            }
            (SplitError::AddressMismatch, false) => {
                "The address does NOT come from your key half. Do not use this address!"
            }
            (SplitError::ProofMismatch, true) => {
                "Присланная половина не совпадает с той, что вы проверяли раньше."
            }
            (SplitError::ProofMismatch, false) => {
                "The received half does not match the one you verified earlier."
            }
        }
    }
}

impl fmt::Display for SplitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text(false))
    }
}

// ---------------------------------------------------------------------------
// Адреса
// ---------------------------------------------------------------------------

fn hash160(data: &[u8]) -> [u8; 20] {
    use ripemd::Ripemd160;
    let mut out = [0u8; 20];
    out.copy_from_slice(&Ripemd160::digest(Sha256::digest(data)));
    out
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    use tiny_keccak::{Hasher, Keccak};
    let mut out = [0u8; 32];
    let mut k = Keccak::v256();
    k.update(data);
    k.finalize(&mut out);
    out
}

fn base58check(version: u8, body: &[u8]) -> String {
    let mut payload = Vec::with_capacity(1 + body.len());
    payload.push(version);
    payload.extend_from_slice(body);
    bs58::encode(payload).with_check().into_string()
}

fn eip55(addr20: &[u8]) -> String {
    let lower: String = addr20.iter().map(|b| format!("{b:02x}")).collect();
    let hash = keccak256(lower.as_bytes());
    let mut out = String::from("0x");
    for (i, c) in lower.chars().enumerate() {
        let nibble = if i % 2 == 0 { hash[i / 2] >> 4 } else { hash[i / 2] & 0xf };
        out.push(if c.is_ascii_alphabetic() && nibble >= 8 { c.to_ascii_uppercase() } else { c });
    }
    out
}

/// BIP341/BIP86: выходной ключ Taproot без дерева скриптов.
fn taproot_output(pk: &PublicKey) -> Option<[u8; 32]> {
    let (internal, _) = pk.x_only_public_key();
    let tag = Sha256::digest(b"TapTweak");
    let mut h = Sha256::new();
    h.update(tag);
    h.update(tag);
    h.update(internal.serialize());
    let tweak = Scalar::from_be_bytes(h.finalize().into()).ok()?;
    let (output, _) = internal.add_tweak(secp(), &tweak).ok()?;
    Some(output.serialize())
}

/// Адрес сети для публичного ключа.
pub fn address(net: Network, pk: &PublicKey) -> String {
    match net.encoding() {
        Encoding::Tron => {
            let hash = keccak256(&pk.serialize_uncompressed()[1..]);
            base58check(0x41, &hash[12..])
        }
        Encoding::Evm => eip55(&keccak256(&pk.serialize_uncompressed()[1..])[12..]),
        Encoding::Base58 { version, p2sh, ripple } => {
            let mut h = hash160(&pk.serialize());
            if p2sh {
                let mut redeem = vec![0x00, 0x14];
                redeem.extend_from_slice(&h);
                h = hash160(&redeem);
            }
            if ripple {
                let mut payload = vec![version];
                payload.extend_from_slice(&h);
                bs58::encode(payload).with_alphabet(bs58::Alphabet::RIPPLE).with_check().into_string()
            } else {
                base58check(version, &h)
            }
        }
        Encoding::SegwitV0 { hrp } => {
            let hrp = Hrp::parse(hrp).expect("valid hrp");
            segwit::encode_v0(hrp, &hash160(&pk.serialize())).expect("20-byte witness program")
        }
        Encoding::Taproot => match taproot_output(pk) {
            Some(x) => segwit::encode_v1(hrp::BC, &x).expect("32-byte witness program"),
            // Хеш-твик ≥ n: вероятность ~2^-128, такой адрес не существует.
            None => String::new(),
        },
        Encoding::Bech32 { hrp } => {
            let hrp = Hrp::parse(hrp).expect("valid hrp");
            bech32::encode::<Bech32>(hrp, &hash160(&pk.serialize())).expect("20-byte program")
        }
    }
}

/// Приватный ключ в формате, который принимают кошельки сети: WIF (сжатый
/// ключ) для BTC, LTC, DOGE и DASH, hex для остальных.
pub fn export_secret(net: Network, k: &SecretKey) -> String {
    match net.wif_version() {
        Some(version) => {
            let mut payload = Vec::with_capacity(33);
            payload.extend_from_slice(&k.secret_bytes());
            payload.push(0x01);
            base58check(version, &payload)
        }
        None => hex(&k.secret_bytes()),
    }
}

// ---------------------------------------------------------------------------
// Коды: TAG:поля:контрольная сумма (первые 4 байта SHA-256 всего до неё)
// ---------------------------------------------------------------------------

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

fn with_checksum(body: String) -> String {
    let sum = Sha256::digest(body.as_bytes());
    format!("{body}:{}", hex(&sum[..4]))
}

/// Разбирает код с тегом `tag`: пробелы и переносы строк (от копирования)
/// игнорируются, контрольная сумма проверяется.
fn parse_fields(code: &str, tag: &str, n_fields: usize) -> Result<Vec<String>, SplitError> {
    let clean: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    let (body, sum) = clean.rsplit_once(':').ok_or(SplitError::BadFormat)?;
    let fields: Vec<&str> = body.split(':').collect();
    if fields.len() != n_fields + 1 || !fields[0].eq_ignore_ascii_case(tag) {
        return Err(SplitError::BadFormat);
    }
    let expected = Sha256::digest(format!("{tag}:{}", fields[1..].join(":")).as_bytes());
    if !sum.eq_ignore_ascii_case(&hex(&expected[..4])) {
        return Err(SplitError::BadChecksum);
    }
    Ok(fields[1..].iter().map(|s| s.to_string()).collect())
}

fn parse_public(s: &str) -> Result<PublicKey, SplitError> {
    let bytes = unhex(&s.to_ascii_lowercase()).ok_or(SplitError::BadKey)?;
    if bytes.len() != 33 {
        return Err(SplitError::BadKey);
    }
    PublicKey::from_slice(&bytes).map_err(|_| SplitError::BadKey)
}

/// Номер заявки: первые 4 байта SHA-256 публичной половины покупателя.
pub fn order_id(a_pub: &PublicKey) -> String {
    hex(&Sha256::digest(a_pub.serialize())[..4]).to_ascii_uppercase()
}

/// Заявка покупателя: его публичная половина `A`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub a_pub: PublicKey,
}

impl Request {
    pub fn encode(&self) -> String {
        with_checksum(format!("{REQ_TAG}:{}", hex(&self.a_pub.serialize())))
    }

    pub fn decode(code: &str) -> Result<Request, SplitError> {
        let f = parse_fields(code, REQ_TAG, 1)?;
        Ok(Request { a_pub: parse_public(&f[0])? })
    }

    pub fn order_id(&self) -> String {
        order_id(&self.a_pub)
    }
}

/// Подтверждение адреса до оплаты: адрес и публичная половина продавца `B`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proof {
    pub order_id: String,
    pub net: Network,
    pub address: String,
    pub b_pub: PublicKey,
}

impl Proof {
    pub fn encode(&self) -> String {
        with_checksum(format!(
            "{PROOF_TAG}:{}:{}:{}:{}",
            self.order_id,
            self.net.tag(),
            self.address,
            hex(&self.b_pub.serialize())
        ))
    }

    pub fn decode(code: &str) -> Result<Proof, SplitError> {
        let f = parse_fields(code, PROOF_TAG, 4)?;
        Ok(Proof {
            order_id: f[0].to_ascii_uppercase(),
            net: Network::from_tag(&f[1]).ok_or(SplitError::UnknownNetwork)?,
            address: f[2].clone(),
            b_pub: parse_public(&f[3])?,
        })
    }

    /// Проверка покупателем: адрес действительно получается из `A + B`.
    pub fn verify(&self, request: &Request) -> Result<(), SplitError> {
        if self.order_id != request.order_id() {
            return Err(SplitError::WrongOrder);
        }
        let q = request.a_pub.combine(&self.b_pub).map_err(|_| SplitError::BadKey)?;
        if address(self.net, &q) != self.address {
            return Err(SplitError::AddressMismatch);
        }
        Ok(())
    }
}

/// Половина продавца после оплаты: число `b`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyHalf {
    pub order_id: String,
    pub net: Network,
    pub address: String,
    pub b: SecretKey,
}

impl KeyHalf {
    /// Код создаёт программа продавца; покупателю он нужен только в тестах.
    #[allow(dead_code)]
    pub fn encode(&self) -> String {
        with_checksum(format!(
            "{KEY_TAG}:{}:{}:{}:{}",
            self.order_id,
            self.net.tag(),
            self.address,
            hex(&self.b.secret_bytes())
        ))
    }

    pub fn decode(code: &str) -> Result<KeyHalf, SplitError> {
        let f = parse_fields(code, KEY_TAG, 4)?;
        let bytes = unhex(&f[3].to_ascii_lowercase()).ok_or(SplitError::BadKey)?;
        Ok(KeyHalf {
            order_id: f[0].to_ascii_uppercase(),
            net: Network::from_tag(&f[1]).ok_or(SplitError::UnknownNetwork)?,
            address: f[2].clone(),
            b: SecretKey::from_slice(&bytes).map_err(|_| SplitError::BadKey)?,
        })
    }

    /// Публичное подтверждение этой половины (для отправки до оплаты).
    pub fn proof(&self) -> Proof {
        Proof {
            order_id: self.order_id.clone(),
            net: self.net,
            address: self.address.clone(),
            b_pub: PublicKey::from_secret_key(secp(), &self.b),
        }
    }

    /// Проверка продавцом перед отправкой и покупателем до сборки: адрес
    /// получается из `A + b·G`.
    pub fn verify(&self, request: &Request) -> Result<(), SplitError> {
        self.proof().verify(request)
    }

    /// Сборка кошелька покупателем: `k = a + b mod n`. Если покупатель
    /// раньше проверил подтверждение, половина должна с ним совпасть.
    pub fn combine(&self, a: &SecretKey, verified: Option<&Proof>) -> Result<SecretKey, SplitError> {
        let request = Request { a_pub: PublicKey::from_secret_key(secp(), a) };
        if let Some(p) = verified {
            if *p != self.proof() {
                return Err(SplitError::ProofMismatch);
            }
        }
        self.verify(&request)?;
        let k = a
            .add_tweak(&Scalar::from_be_bytes(self.b.secret_bytes()).map_err(|_| SplitError::BadKey)?)
            .map_err(|_| SplitError::BadKey)?;
        // Окончательная проверка — уже из полного приватного ключа.
        if address(self.net, &PublicKey::from_secret_key(secp(), &k)) != self.address {
            return Err(SplitError::AddressMismatch);
        }
        Ok(k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(n: u8) -> SecretKey {
        let mut b = [0u8; 32];
        b[31] = n;
        SecretKey::from_slice(&b).unwrap()
    }

    /// Общеизвестные адреса ключа 1 (точка G).
    #[test]
    fn known_addresses_for_key_one() {
        let g = PublicKey::from_secret_key(secp(), &key(1));
        assert_eq!(address(Network::Eth, &g), "0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf");
        assert_eq!(address(Network::BtcP2pkh, &g), "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH");
        assert_eq!(address(Network::BtcP2sh, &g), "3JvL6Ymt8MVWiCNHC7oWU6nLeHNJKLZGLN");
        assert_eq!(address(Network::BtcP2wpkh, &g), "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4");
        assert_eq!(
            address(Network::BtcP2tr, &g),
            "bc1pmfr3p9j00pfxjh0zmgp99y8zftmd3s5pmedqhyptwy6lm87hf5sspknck9"
        );
        assert_eq!(
            export_secret(Network::BtcP2pkh, &key(1)),
            "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn"
        );
        assert!(address(Network::Tron, &g).starts_with('T'));
    }

    /// Новые сети на ключе 1 (HASH160 = 751e76e8…): адрес раскодирован
    /// эталонными bs58/bech32 обратно в «версия || HASH160» и начинается с
    /// постоянной части сети; EVM-сети дают адрес Ethereum.
    #[test]
    fn new_networks_for_key_one() {
        let g = PublicKey::from_secret_key(secp(), &key(1));
        let h = unhex("751e76e8199196d454941c45d1b3a323f1433bd6").unwrap();
        let eth = address(Network::Eth, &g);
        for net in Network::ALL.into_iter().filter(|n| n.is_evm()) {
            assert_eq!(address(net, &g), eth, "{net:?}");
            assert_eq!(export_secret(net, &key(1)), hex(&key(1).secret_bytes()));
        }
        for (net, version, first) in [
            (Network::LtcP2pkh, 0x30, 'L'),
            (Network::Doge, 0x1e, 'D'),
            (Network::Dash, 0x4c, 'X'),
        ] {
            let a = address(net, &g);
            let raw = bs58::decode(&a).with_check(None).into_vec().unwrap();
            assert_eq!((raw[0], &raw[1..]), (version, &h[..]), "{a}");
            assert!(a.starts_with(first) && a.len() == 34, "{a}");
        }
        let m = address(Network::LtcP2sh, &g);
        assert!(m.starts_with('M') && bs58::decode(&m).with_check(Some(0x32)).into_vec().is_ok(), "{m}");
        assert_eq!(&address(Network::LtcP2wpkh, &g)[..37], "ltc1qw508d6qejxtdg4y5r3zarvary0c5xw7k");
        for (net, hrp) in [(Network::Cosmos, "cosmos"), (Network::Osmosis, "osmo"), (Network::Noble, "noble")] {
            let a = address(net, &g);
            let (got_hrp, data) = bech32::decode(&a).unwrap();
            assert_eq!((got_hrp.as_str(), &data[..]), (hrp, &h[..]), "{a}");
            assert!(a.starts_with(&format!("{hrp}1w508d6qejxtdg4y5r3zarvary0c5xw7k")), "{a}");
        }
        // WIF своих сетей: версия || ключ || 0x01.
        for (net, version) in [(Network::LtcP2pkh, 0xb0), (Network::Doge, 0x9e), (Network::Dash, 0xcc)] {
            let raw = bs58::decode(export_secret(net, &key(1))).with_check(Some(version)).into_vec().unwrap();
            assert_eq!(&raw[1..], &[&key(1).secret_bytes()[..], &[1]].concat()[..]);
        }
    }

    /// XRP Ledger: известный адрес генезис-аккаунта по его публичному ключу.
    #[test]
    fn xrp_genesis_address() {
        let pk = PublicKey::from_slice(
            &unhex("0330e7fc9d56bb25d6893ba3f317ae5bcf33b3291bd63db32654a313222f7fd020").unwrap(),
        )
        .unwrap();
        assert_eq!(address(Network::Xrp, &pk), "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh");
        assert_eq!(export_secret(Network::Xrp, &key(1)), hex(&key(1).secret_bytes()));
    }

    /// Метки и названия сетей уникальны, а старые метки не изменились.
    #[test]
    fn tags_are_unique_and_stable() {
        let tags: std::collections::HashSet<_> = Network::ALL.iter().map(|n| n.tag()).collect();
        let labels: std::collections::HashSet<_> = Network::ALL.iter().map(|n| n.label()).collect();
        assert_eq!((tags.len(), labels.len()), (Network::ALL.len(), Network::ALL.len()));
        let old: Vec<_> = [
            Network::Tron,
            Network::Eth,
            Network::BtcP2pkh,
            Network::BtcP2sh,
            Network::BtcP2wpkh,
            Network::BtcP2tr,
        ]
        .iter()
        .map(|n| n.tag())
        .collect();
        assert_eq!(old, ["TRON", "ETH", "BTC-P2PKH", "BTC-P2SH", "BTC-P2WPKH", "BTC-P2TR"]);
    }

    /// Полный цикл: заявка → подтверждение → половина → кошелёк, во всех сетях.
    #[test]
    fn full_cycle_every_network() {
        for net in Network::ALL {
            let a = key(7);
            let request = Request { a_pub: PublicKey::from_secret_key(secp(), &a) };
            let request = Request::decode(&request.encode()).unwrap();
            let b = key(200);
            let q = request.a_pub.combine(&PublicKey::from_secret_key(secp(), &b)).unwrap();
            let half = KeyHalf {
                order_id: request.order_id(),
                net,
                address: address(net, &q),
                b,
            };
            let proof = Proof::decode(&half.proof().encode()).unwrap();
            proof.verify(&request).unwrap();
            let half = KeyHalf::decode(&half.encode()).unwrap();
            let k = half.combine(&a, Some(&proof)).unwrap();
            // 7 + 200 = 207.
            assert_eq!(k, key(207));
            assert_eq!(address(net, &PublicKey::from_secret_key(secp(), &k)), half.address);
        }
    }

    #[test]
    fn damaged_or_foreign_codes_are_rejected() {
        let a = key(3);
        let request = Request { a_pub: PublicKey::from_secret_key(secp(), &a) };
        let code = request.encode();
        // Переносы строк и пробелы от копирования не мешают.
        let wrapped = format!("  {}\n{} ", &code[..20], &code[20..]);
        assert_eq!(Request::decode(&wrapped).unwrap(), request);
        // Одна изменённая цифра ключа ловится контрольной суммой.
        let mut damaged = code.clone().into_bytes();
        damaged[10] = if damaged[10] == b'a' { b'b' } else { b'a' };
        assert!(Request::decode(std::str::from_utf8(&damaged).unwrap()).is_err());

        let b = key(9);
        let q = request.a_pub.combine(&PublicKey::from_secret_key(secp(), &b)).unwrap();
        let half = KeyHalf {
            order_id: request.order_id(),
            net: Network::Tron,
            address: address(Network::Tron, &q),
            b,
        };
        // Подменённый адрес не проходит проверку.
        let mut forged = half.proof();
        forged.address = address(Network::Tron, &PublicKey::from_secret_key(secp(), &key(5)));
        assert_eq!(forged.verify(&request), Err(SplitError::AddressMismatch));
        // Чужая заявка.
        let other = Request { a_pub: PublicKey::from_secret_key(secp(), &key(4)) };
        assert_eq!(half.proof().verify(&other), Err(SplitError::WrongOrder));
        // Половина не от того подтверждения.
        let mut other_proof = half.proof();
        other_proof.b_pub = PublicKey::from_secret_key(secp(), &key(10));
        assert_eq!(half.combine(&a, Some(&other_proof)), Err(SplitError::ProofMismatch));
        // Неверный секрет покупателя.
        assert!(half.combine(&key(4), None).is_err());
        // Код подтверждения нельзя выдать за код ключа и наоборот.
        assert!(KeyHalf::decode(&half.proof().encode()).is_err());
        assert!(Proof::decode(&half.encode()).is_err());
    }
}
