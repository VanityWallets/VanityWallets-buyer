//! Встраивает в exe Windows-ресурсы: иконку (Проводник, панель задач,
//! Диспетчер задач), сведения о версии (описание процесса в Диспетчере) и
//! манифест приложения (обычные права пользователя, поддерживаемые версии
//! Windows): exe без манифеста и описания антивирусы чаще считают
//! подозрительным. Файл .res пишется здесь же из `assets/icon.ico`, без
//! внешних утилит (rc.exe/windres не нужны) и без зависимостей.

use std::env;
use std::fs;
use std::path::PathBuf;

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
const RT_MANIFEST: u16 = 24;
const LANG_EN_US: u16 = 0x0409;
const PRODUCT: &str = "Vanity Buyer";
const DESCRIPTION: &str = "Offline split-key wallet assembly, no network code";

/// Манифест приложения: запуск с правами обычного пользователя (без запроса
/// UAC) и поддерживаемые версии Windows (7 — 11). Настройки DPI не задаются:
/// их, как и раньше, выставляет оконная библиотека при запуске.
const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="VanityBuyer" version="1.0.0.0" processorArchitecture="amd64"/>
  <description>Vanity Buyer</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}"/>
      <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}"/>
      <supportedOS Id="{35138b9a-5d96-4fbd-8e2d-a2440225f93a}"/>
    </application>
  </compatibility>
</assembly>
"#;

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        println!("cargo:warning=иконка exe встраивается только для MSVC-сборки");
        return;
    }
    let ico = fs::read("assets/icon.ico").expect("assets/icon.ico");
    let mut res = Vec::new();
    push_resource(&mut res, 0, 0, 0, &[]);
    let group = push_icons(&mut res, &ico);
    push_resource(&mut res, RT_GROUP_ICON, 1, 0x1030, &group);
    push_resource(&mut res, RT_VERSION, 1, 0x0030, &version_info());
    // ID 1 — манифест процесса (CREATEPROCESS_MANIFEST_RESOURCE_ID).
    push_resource(&mut res, RT_MANIFEST, 1, 0x1030, MANIFEST.as_bytes());

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("app.res");
    fs::write(&out, res).expect("write app.res");
    // link.exe принимает .res напрямую.
    println!("cargo:rustc-link-arg-bins={}", out.display());
}

fn u16le(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn u32le(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn pad4(v: &mut Vec<u8>) {
    while v.len() % 4 != 0 {
        v.push(0);
    }
}

/// Запись .res: заголовок RESOURCEHEADER с числовыми типом и именем, данные,
/// выравнивание по 4 байтам.
fn push_resource(res: &mut Vec<u8>, kind: u16, id: u16, flags: u16, data: &[u8]) {
    u32le(res, data.len() as u32);
    u32le(res, 32);
    u16le(res, 0xffff);
    u16le(res, kind);
    u16le(res, 0xffff);
    u16le(res, id);
    u32le(res, 0);
    u16le(res, flags);
    u16le(res, if kind == 0 { 0 } else { LANG_EN_US });
    u32le(res, 0);
    u32le(res, 0);
    res.extend_from_slice(data);
    pad4(res);
}

/// Каждое изображение ICO — отдельный RT_ICON; возвращает каталог
/// RT_GROUP_ICON (как ICONDIR, но с номером ресурса вместо смещения).
fn push_icons(res: &mut Vec<u8>, ico: &[u8]) -> Vec<u8> {
    let rd16 = |o: usize| u16::from_le_bytes([ico[o], ico[o + 1]]);
    let rd32 = |o: usize| u32::from_le_bytes([ico[o], ico[o + 1], ico[o + 2], ico[o + 3]]);
    assert!(rd16(0) == 0 && rd16(2) == 1, "assets/icon.ico: не ICO");
    let count = rd16(4);
    let mut group = Vec::new();
    u16le(&mut group, 0);
    u16le(&mut group, 1);
    u16le(&mut group, count);
    for i in 0..count {
        let e = 6 + 16 * i as usize;
        let (size, offset) = (rd32(e + 8) as usize, rd32(e + 12) as usize);
        let id = i + 1;
        push_resource(res, RT_ICON, id, 0x1010, &ico[offset..offset + size]);
        group.extend_from_slice(&ico[e..e + 8]);
        u32le(&mut group, size as u32);
        u16le(&mut group, id);
    }
    group
}

fn utf16z(s: &str) -> Vec<u8> {
    s.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// Узел VS_VERSIONINFO: wLength, wValueLength, wType, ключ, значение, дети.
fn node(key: &str, value: &[u8], value_len: u16, text: bool, children: &[Vec<u8>]) -> Vec<u8> {
    let mut v = vec![0, 0];
    u16le(&mut v, value_len);
    u16le(&mut v, u16::from(text));
    v.extend_from_slice(&utf16z(key));
    pad4(&mut v);
    v.extend_from_slice(value);
    for child in children {
        pad4(&mut v);
        v.extend_from_slice(child);
    }
    let len = v.len() as u16;
    v[..2].copy_from_slice(&len.to_le_bytes());
    v
}

fn version_info() -> Vec<u8> {
    let parts: Vec<u16> = env::var("CARGO_PKG_VERSION")
        .unwrap_or_default()
        .split('.')
        .map(|p| p.parse().unwrap_or(0))
        .chain(std::iter::repeat(0))
        .take(4)
        .collect();
    let ms = u32::from(parts[0]) << 16 | u32::from(parts[1]);
    let ls = u32::from(parts[2]) << 16 | u32::from(parts[3]);
    let version = format!("{}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3]);

    let mut fixed = Vec::new();
    for x in [
        0xfeef_04bd, // signature
        0x0001_0000, // structure version
        ms,
        ls,
        ms,
        ls,
        0x3f,      // flags mask
        0,         // flags
        0x0004_0004, // VOS_NT_WINDOWS32
        1,         // VFT_APP
        0,
        0,
        0,
    ] {
        u32le(&mut fixed, x);
    }

    let string = |key: &str, value: &str| {
        let data = utf16z(value);
        node(key, &data, (data.len() / 2) as u16, true, &[])
    };
    let exe = format!("{}.exe", env::var("CARGO_PKG_NAME").unwrap_or_default());
    let table = node(
        "040904B0",
        &[],
        0,
        true,
        &[
            string("CompanyName", PRODUCT),
            // Это имя показывает Диспетчер задач — оставлено прежним.
            string("FileDescription", PRODUCT),
            string("ProductName", PRODUCT),
            string("LegalCopyright", "Open source"),
            string("Comments", DESCRIPTION),
            string("FileVersion", &version),
            string("ProductVersion", &version),
            string("OriginalFilename", &exe),
            string("InternalName", &exe),
        ],
    );
    let strings = node("StringFileInfo", &[], 0, true, &[table]);
    let mut translation = Vec::new();
    u16le(&mut translation, LANG_EN_US);
    u16le(&mut translation, 1200);
    let var = node("Translation", &translation, 4, false, &[]);
    let vars = node("VarFileInfo", &[], 0, true, &[var]);
    node("VS_VERSION_INFO", &fixed, fixed.len() as u16, false, &[strings, vars])
}
