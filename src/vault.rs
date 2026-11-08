//! Заявки покупателя. Секрет `a` каждой заявки хранится только на этом
//! компьютере, в отдельном файле папки `orders`. Без этого файла собрать
//! заказанный кошелёк невозможно, поэтому его нужно хранить до получения
//! кошелька и никому не передавать.

use crate::split_key::{self, Network, Request};
use rand::rngs::OsRng;
use rand::RngCore;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

const FORMAT: &str = "pokupatel-order-2";

#[derive(Serialize, Deserialize)]
struct OrderFile {
    #[serde(rename = "Format")]
    format: String,
    #[serde(rename = "OrderId")]
    order_id: String,
    /// Код заявки (публичная половина) — его можно отправлять кому угодно.
    #[serde(rename = "Request")]
    request: String,
    #[serde(rename = "Created")]
    created: u64,
    /// Секретная половина покупателя (hex). Никому не передавать.
    #[serde(rename = "SecretHalf")]
    secret_half: String,
    /// Что заказано: сеть, начало и конец адреса.
    #[serde(rename = "Network", default, skip_serializing_if = "Option::is_none")]
    network: Option<String>,
    #[serde(rename = "WantedStart", default)]
    wanted_start: String,
    #[serde(rename = "WantedEnd", default)]
    wanted_end: String,
    /// Подтверждение адреса, проверенное на шаге 2.
    #[serde(rename = "VerifiedProof", default, skip_serializing_if = "Option::is_none")]
    verified_proof: Option<String>,
    /// Адрес кошелька, собранного на шаге 3 (сам ключ здесь не хранится).
    #[serde(rename = "AssembledAddress", default, skip_serializing_if = "Option::is_none")]
    assembled_address: Option<String>,
    #[serde(rename = "Note")]
    note: String,
}

/// Что покупатель заказал.
#[derive(Clone)]
pub struct Wish {
    pub net: Network,
    pub start: String,
    pub end: String,
}

#[derive(Clone)]
pub struct Order {
    pub id: String,
    pub request: String,
    pub created: u64,
    pub wish: Option<Wish>,
    pub verified_proof: Option<String>,
    pub assembled_address: Option<String>,
    path: PathBuf,
}

impl Order {
    fn from_file(f: &OrderFile, path: PathBuf) -> Order {
        Order {
            id: f.order_id.clone(),
            request: f.request.clone(),
            created: f.created,
            wish: f.network.as_deref().and_then(Network::from_tag).map(|net| Wish {
                net,
                start: f.wanted_start.clone(),
                end: f.wanted_end.clone(),
            }),
            verified_proof: f.verified_proof.clone(),
            assembled_address: f.assembled_address.clone(),
            path,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum VaultError {
    Damaged,
    Io(String),
}

impl VaultError {
    pub fn text(&self, ru: bool) -> String {
        match (self, ru) {
            (VaultError::Damaged, true) => "Файл заявки повреждён или изменён.".into(),
            (VaultError::Damaged, false) => "The request file is damaged or was modified.".into(),
            (VaultError::Io(e), true) => format!("Ошибка файла: {e}"),
            (VaultError::Io(e), false) => format!("File error: {e}"),
        }
    }
}

/// Папка программы: рядом с exe, а если туда нельзя писать —
/// %LOCALAPPDATA%\VanityBuyer.
pub fn data_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let near_exe = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf));
        if let Some(dir) = near_exe.filter(|d| writable(d)) {
            return dir;
        }
        match std::env::var_os("LOCALAPPDATA") {
            Some(base) => PathBuf::from(base).join("VanityBuyer"),
            None => PathBuf::from("."),
        }
    })
    .clone()
}

fn writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".write_test_{}", std::process::id()));
    match OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(_) => fs::remove_file(&probe).is_ok(),
        Err(_) => false,
    }
}

pub fn orders_dir() -> PathBuf {
    data_dir().join("orders")
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

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn write_new(path: &Path, text: &str) -> io::Result<()> {
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
    f.write_all(text.as_bytes())?;
    f.sync_all()
}

/// Новая заявка в папке `dir` со случайной секретной половиной.
pub fn create_in(dir: &Path, wish: &Wish) -> Result<Order, VaultError> {
    fs::create_dir_all(dir).map_err(|e| VaultError::Io(e.to_string()))?;
    let secp = Secp256k1::new();
    loop {
        let mut a_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut a_bytes);
        let Ok(a) = SecretKey::from_slice(&a_bytes) else {
            continue;
        };
        let request = Request { a_pub: PublicKey::from_secret_key(&secp, &a) };
        let code = request.encode();
        let id = request.order_id();
        let path = dir.join(format!("{id}.json"));
        if path.exists() {
            a_bytes.zeroize();
            continue;
        }

        let created = now();
        let mut file = OrderFile {
            format: FORMAT.into(),
            order_id: id.clone(),
            request: code.clone(),
            created,
            secret_half: split_key::hex(&a_bytes),
            network: Some(wish.net.tag().to_string()),
            wanted_start: wish.start.clone(),
            wanted_end: wish.end.clone(),
            verified_proof: None,
            assembled_address: None,
            note: "Секретная половина ключа заказанного кошелька. Никому не передавайте этот файл и \
                   храните его до получения кошелька: без него кошелёк собрать невозможно. / \
                   Secret half of the ordered wallet key. Never share this file and keep it until \
                   you receive the wallet: without it the wallet cannot be assembled."
                .into(),
        };
        a_bytes.zeroize();
        let text = serde_json::to_string_pretty(&file).map_err(|e| VaultError::Io(e.to_string()));
        file.secret_half.zeroize();
        let mut text = text?;
        let written = write_new(&path, &text).map_err(|e| VaultError::Io(e.to_string()));
        text.zeroize();
        written?;
        return Ok(Order::from_file(&file, path));
    }
}

pub fn create(wish: &Wish) -> Result<Order, VaultError> {
    create_in(&orders_dir(), wish)
}

fn read_file(path: &Path) -> Option<OrderFile> {
    let text = fs::read_to_string(path).ok()?;
    let file: OrderFile = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    (file.format == FORMAT).then_some(file)
}

/// Все заявки папки `dir`, новые сверху.
pub fn list_in(dir: &Path) -> Vec<Order> {
    let mut orders: Vec<Order> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| {
            let path = e.path();
            let mut f = read_file(&path)?;
            f.secret_half.zeroize();
            Some(Order::from_file(&f, path))
        })
        .collect();
    orders.sort_by(|a, b| b.created.cmp(&a.created));
    orders
}

pub fn list() -> Vec<Order> {
    list_in(&orders_dir())
}

/// Секретная половина заявки; проверяется, что она соответствует коду заявки.
pub fn secret(order: &Order) -> Result<SecretKey, VaultError> {
    let mut f = read_file(&order.path).ok_or(VaultError::Damaged)?;
    let bytes = unhex(&f.secret_half);
    f.secret_half.zeroize();
    let mut bytes = bytes.ok_or(VaultError::Damaged)?;
    let a = SecretKey::from_slice(&bytes).map_err(|_| VaultError::Damaged);
    bytes.zeroize();
    let a = a?;
    let request = Request::decode(&f.request).map_err(|_| VaultError::Damaged)?;
    if PublicKey::from_secret_key(&Secp256k1::new(), &a) != request.a_pub {
        return Err(VaultError::Damaged);
    }
    Ok(a)
}

/// Запоминает проверенное подтверждение адреса в файле заявки.
pub fn save_verified_proof(order: &Order, proof_code: &str) -> Result<(), VaultError> {
    update(order, |f| f.verified_proof = Some(proof_code.to_string()))
}

/// Запоминает адрес собранного кошелька (без ключа) — для статуса заявки.
pub fn save_assembled(order: &Order, address: &str) -> Result<(), VaultError> {
    update(order, |f| f.assembled_address = Some(address.to_string()))
}

fn update(order: &Order, change: impl FnOnce(&mut OrderFile)) -> Result<(), VaultError> {
    let mut f = read_file(&order.path).ok_or(VaultError::Damaged)?;
    change(&mut f);
    let text = serde_json::to_string_pretty(&f).map_err(|e| VaultError::Io(e.to_string()));
    f.secret_half.zeroize();
    let mut text = text?;
    let tmp = order.path.with_extension("json.tmp");
    let _ = fs::remove_file(&tmp);
    let written = write_new(&tmp, &text).map_err(|e| VaultError::Io(e.to_string()));
    text.zeroize();
    written?;
    fs::rename(&tmp, &order.path).map_err(|e| VaultError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wish() -> Wish {
        Wish { net: Network::Tron, start: "Pet".into(), end: String::new() }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pokupatel_test_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn create_list_and_read_secret() {
        let dir = temp_dir("vault");
        let order = create_in(&dir, &wish()).unwrap();
        let listed = list_in(&dir);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, order.id);
        let w = listed[0].wish.clone().unwrap();
        assert_eq!((w.net, w.start.as_str()), (Network::Tron, "Pet"));
        save_assembled(&listed[0], "TAddr").unwrap();
        assert_eq!(list_in(&dir)[0].assembled_address.as_deref(), Some("TAddr"));

        let a = secret(&listed[0]).unwrap();
        let request = Request::decode(&order.request).unwrap();
        assert_eq!(PublicKey::from_secret_key(&Secp256k1::new(), &a), request.a_pub);

        save_verified_proof(&listed[0], "VGPROOF1:test").unwrap();
        assert_eq!(list_in(&dir)[0].verified_proof.as_deref(), Some("VGPROOF1:test"));
        assert!(secret(&list_in(&dir)[0]).is_ok());
        fs::remove_dir_all(&dir).unwrap();
    }

    /// Подменённый код заявки в файле обнаруживается: секрет ему не соответствует.
    #[test]
    fn swapped_request_is_detected() {
        let dir = temp_dir("swap");
        let one = create_in(&dir, &wish()).unwrap();
        let two = create_in(&dir, &wish()).unwrap();
        let mut f = read_file(&one.path).unwrap();
        f.request = two.request.clone();
        fs::write(&one.path, serde_json::to_string(&f).unwrap()).unwrap();
        let order = list_in(&dir).into_iter().find(|o| o.path == one.path).unwrap();
        assert_eq!(secret(&order).unwrap_err(), VaultError::Damaged);
        fs::remove_dir_all(&dir).unwrap();
    }
}
