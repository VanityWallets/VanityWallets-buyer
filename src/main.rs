//! Программа покупателя vanity-кошелька с разделённым ключом (TRON, ETH и
//! EVM-сети, BTC, LTC, DOGE, DASH, XRP, Cosmos-сети).
//! Работает полностью офлайн: никакого сетевого кода. Секрет покупателя не
//! покидает этот компьютер.
//!
//! «Как это работает» — простое объяснение и как самому убедиться в честности.
//! 1. «Заявка» — что заказать; программа создаёт секрет и код для продавца.
//! 2. «Проверка адреса» — до оплаты убедиться, что адрес собран из своей
//!    половины ключа и подходит под заказ.
//! 3. «Кошелёк» — после оплаты собрать полный приватный ключ.
//! На шагах 2 и 3 программа показывает каждую проверку отдельно.

#![windows_subsystem = "windows"]

mod split_key;
mod vault;

use eframe::egui::{self, Color32, RichText, Vec2};
use split_key::{Encoding, KeyHalf, Network, Proof, Request, SplitError};
use std::fs::OpenOptions;
use std::io::Write;
use vault::Wish;
use zeroize::Zeroize;

const ICON_RGBA: &[u8] = include_bytes!("../assets/icon_256.rgba");

// Тёмная тема программы (одинаковая на любом компьютере).
const BG: Color32 = Color32::from_rgb(11, 14, 18);
const SIDEBAR_BG: Color32 = Color32::from_rgb(15, 19, 24);
const CARD_BG: Color32 = Color32::from_rgb(20, 26, 33);
const CARD_BORDER: Color32 = Color32::from_rgb(37, 45, 56);
const INPUT_BG: Color32 = Color32::from_rgb(15, 19, 24);
const INPUT_BORDER: Color32 = Color32::from_rgb(52, 62, 75);
const BUTTON_BG: Color32 = Color32::from_rgb(26, 33, 42);
const TEXT: Color32 = Color32::from_rgb(232, 236, 242);
const CODE_TEXT: Color32 = Color32::from_rgb(213, 219, 227);
const HINT_TEXT: Color32 = Color32::from_rgb(163, 173, 187);
const FAINT_TEXT: Color32 = Color32::from_rgb(125, 136, 150);
const ACCENT: Color32 = Color32::from_rgb(61, 220, 132);
/// Текст на кнопке цвета ACCENT.
const ON_ACCENT: Color32 = Color32::from_rgb(6, 33, 15);
const NAV_ACTIVE: Color32 = Color32::from_rgb(23, 42, 32);
const OK_TEXT: Color32 = Color32::from_rgb(123, 235, 172);
const OK_BG: Color32 = Color32::from_rgb(16, 37, 26);
const OK_BORDER: Color32 = Color32::from_rgb(43, 91, 64);
const WARN_TEXT: Color32 = Color32::from_rgb(245, 196, 81);
const WARN_BG: Color32 = Color32::from_rgb(42, 35, 17);
const ERR_TEXT: Color32 = Color32::from_rgb(255, 138, 138);
const ERR_BG: Color32 = Color32::from_rgb(42, 20, 22);
const ERR_BORDER: Color32 = Color32::from_rgb(110, 46, 50);

/// Ширина окна, начиная с которой шаги показываются с подписями, а блоки
/// стоят в две колонки.
const WIDE: f32 = 900.0;

/// Тёмная тема с контрастным текстом и заметными полями ввода: рамка,
/// отдельный фон, подсветка рамки при наборе.
fn apply_style(style: &mut egui::Style) {
    use egui::{FontFamily, FontId, Rounding, Stroke, TextStyle};
    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = CARD_BG;
    v.window_stroke = Stroke::new(1.0, CARD_BORDER);
    v.faint_bg_color = CARD_BG;
    v.extreme_bg_color = INPUT_BG;
    v.code_bg_color = INPUT_BG;
    v.selection.bg_fill = Color32::from_rgb(35, 95, 62);
    // Рамка поля при наборе: светлая, чтобы была заметна на тёмном.
    v.selection.stroke = Stroke::new(1.5, Color32::from_rgb(214, 245, 226));
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, CARD_BORDER);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    for (w, fill, border) in [
        (&mut v.widgets.inactive, BUTTON_BG, INPUT_BORDER),
        (&mut v.widgets.hovered, Color32::from_rgb(34, 42, 53), Color32::from_rgb(110, 122, 138)),
        (&mut v.widgets.active, Color32::from_rgb(40, 50, 62), ACCENT),
        (&mut v.widgets.open, Color32::from_rgb(34, 42, 53), INPUT_BORDER),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = Stroke::new(1.0, border);
        w.fg_stroke = Stroke::new(1.0, TEXT);
        w.rounding = Rounding::same(8.0);
    }
    style.text_styles = [
        (TextStyle::Heading, FontId::new(22.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(14.5, FontFamily::Monospace)),
        (TextStyle::Small, FontId::new(12.5, FontFamily::Proportional)),
    ]
    .into();
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(14.0, 8.0);
    style.spacing.interact_size.y = 36.0;
}

/// Поле ввода: рамка, отступы внутри, моноширинный шрифт, во всю ширину.
fn input<'t>(text: &'t mut String, hint: &str, rows: usize) -> egui::TextEdit<'t> {
    let edit = if rows > 1 { egui::TextEdit::multiline(text).desired_rows(rows) } else { egui::TextEdit::singleline(text) };
    edit.font(egui::TextStyle::Monospace)
        .margin(Vec2::new(12.0, 10.0))
        .desired_width(f32::INFINITY)
        .hint_text(RichText::new(hint).color(FAINT_TEXT))
}

/// При сбое процесс завершается сразу и молча: Windows Error Reporting не
/// снимает дамп памяти, где мог бы оказаться ключ.
#[cfg(windows)]
fn no_crash_reports() {
    use windows::Win32::System::Diagnostics::Debug::{
        SetErrorMode, SetUnhandledExceptionFilter, EXCEPTION_POINTERS, SEM_FAILCRITICALERRORS,
        SEM_NOGPFAULTERRORBOX,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};

    unsafe extern "system" fn on_crash(_: *const EXCEPTION_POINTERS) -> i32 {
        let _ = TerminateProcess(GetCurrentProcess(), 0xC000_0409);
        1
    }
    unsafe {
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
        SetUnhandledExceptionFilter(Some(on_crash));
    }
    std::panic::set_hook(Box::new(|_| unsafe {
        let _ = TerminateProcess(GetCurrentProcess(), 101);
    }));
}

fn main() -> eframe::Result<()> {
    #[cfg(windows)]
    no_crash_reports();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(egui::IconData { rgba: ICON_RGBA.to_vec(), width: 256, height: 256 })
            .with_inner_size([1120.0, 780.0])
            .with_min_inner_size([720.0, 560.0])
            .with_title("Vanity — покупатель / buyer")
            // Без рамки Windows: кнопки окна и перетаскивание — в самой программе.
            .with_decorations(false)
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "Vanity buyer",
        options,
        Box::new(|cc| {
            // Всегда тёмная тема: без этого окно следует теме Windows и на
            // светлой системе становится белым, а поля ввода — незаметными.
            cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
            cc.egui_ctx.style_mut_of(egui::Theme::Dark, apply_style);
            cc.egui_ctx.style_mut_of(egui::Theme::Light, apply_style);
            Ok(Box::new(BuyerApp::new()))
        }),
    )
}

/// Копирует текст в буфер обмена, исключая его из журнала Win+V, облачной
/// синхронизации и сторонних менеджеров буфера.
fn copy_secret(text: &str) -> bool {
    #[cfg(windows)]
    use arboard::SetExtWindows;
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return false;
    };
    let set = clipboard.set();
    #[cfg(windows)]
    let set = set.exclude_from_cloud().exclude_from_history().exclude_from_monitoring();
    set.text(text.to_string()).is_ok()
}

fn clear_secret(text: &str) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if clipboard.get_text().ok().as_deref() == Some(text) {
            let _ = clipboard.clear();
        }
    }
}

/// Дата (UTC) из секунд Unix: ДД.ММ.ГГГГ.
fn date(secs: u64) -> String {
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{d:02}.{m:02}.{y}")
}

/// Сокращение длинной строки: «a985a3e3…7fe68e23».
fn short(s: &str) -> String {
    if s.len() <= 20 {
        s.to_string()
    } else {
        format!("{}…{}", &s[..8], &s[s.len() - 8..])
    }
}

// ---------------------------------------------------------------------------
// Что заказано: постоянное начало адреса сети и допустимые символы
// ---------------------------------------------------------------------------

/// Начало, которое есть у всех адресов сети.
fn fixed_start(net: Network) -> &'static str {
    match net {
        Network::Tron => "T",
        Network::Eth
        | Network::Bsc
        | Network::Polygon
        | Network::Arbitrum
        | Network::Optimism
        | Network::Base
        | Network::Avalanche
        | Network::Linea
        | Network::ZkSync
        | Network::HyperEvm => "0x",
        Network::BtcP2pkh => "1",
        Network::BtcP2sh => "3",
        Network::BtcP2wpkh => "bc1q",
        Network::BtcP2tr => "bc1p",
        Network::LtcP2pkh => "L",
        Network::LtcP2sh => "M",
        Network::LtcP2wpkh => "ltc1q",
        Network::Doge => "D",
        Network::Dash => "X",
        Network::Xrp => "r",
        Network::Cosmos => "cosmos1",
        Network::Osmosis => "osmo1",
        Network::Celestia => "celestia1",
        Network::Dydx => "dydx1",
        Network::Akash => "akash1",
        Network::Juno => "juno1",
        Network::Stargaze => "stars1",
        Network::Noble => "noble1",
    }
}

/// Обычная длина адреса сети — для предпросмотра заказа.
fn typical_len(net: Network) -> usize {
    match net.encoding() {
        Encoding::Tron | Encoding::Base58 { .. } => 34,
        Encoding::Evm => 42,
        // hrp + «1» + 33 символа программы + 6 контрольной суммы.
        Encoding::SegwitV0 { hrp } => hrp.len() + 40,
        Encoding::Taproot => 62,
        // hrp + «1» + 32 символа данных + 6 контрольной суммы.
        Encoding::Bech32 { hrp } => hrp.len() + 39,
    }
}

/// Часть желаемого начала после постоянного (его можно вводить или нет).
fn wanted_body(net: Network, start: &str) -> &str {
    let s = start.trim();
    let fixed = fixed_start(net);
    s.strip_prefix(fixed)
        .or_else(|| if net.is_evm() { s.strip_prefix("0X") } else { None })
        .unwrap_or(s)
}

/// Адреса сети пишутся символами bech32 (bc1…, ltc1…, cosmos1…).
fn is_bech32(net: Network) -> bool {
    matches!(
        net.encoding(),
        Encoding::SegwitV0 { .. } | Encoding::Taproot | Encoding::Bech32 { .. }
    )
}

fn charset_ok(net: Network, s: &str) -> bool {
    // Алфавиты Base58 у BTC и XRP разные, но из одних и тех же символов.
    const B58: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    const BECH32: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    s.chars().all(|c| {
        if net.is_evm() {
            c.is_ascii_hexdigit()
        } else if is_bech32(net) {
            BECH32.contains(c)
        } else {
            B58.contains(c)
        }
    })
}

fn allowed_chars_hint(net: Network, ru: bool) -> &'static str {
    match (net.is_evm(), is_bech32(net), ru) {
        (true, _, true) => "Для адресов 0x (Ethereum, BSC, Polygon и др.) допустимы только 0–9 и a–f (регистр букв важен).",
        (true, _, false) => "0x addresses (Ethereum, BSC, Polygon, etc.) allow only 0–9 and a–f (letter case matters).",
        (_, true, true) => {
            "Для адресов bc1, ltc1 и Cosmos допустимы только строчные буквы и цифры, кроме 1, b, i, o."
        }
        (_, true, false) => {
            "bc1, ltc1 and Cosmos addresses allow only lowercase letters and digits except 1, b, i, o."
        }
        (_, _, true) => "Недопустимы символы 0, O, I и l (их нет в адресах этой сети).",
        (_, _, false) => "The characters 0, O, I and l are not allowed (they never occur in these addresses).",
    }
}

/// Адрес подходит под заказ (так же, как это понимает программа продавца).
fn wish_matches(wish: &Wish, address: &str) -> bool {
    let fixed = fixed_start(wish.net);
    let body = address.strip_prefix(fixed).unwrap_or(address);
    body.starts_with(wanted_body(wish.net, &wish.start)) && address.ends_with(wish.end.trim())
}

fn wish_text(wish: &Wish, ru: bool) -> String {
    let fixed = fixed_start(wish.net);
    let start = wanted_body(wish.net, &wish.start);
    let end = wish.end.trim();
    let mut parts = Vec::new();
    if !start.is_empty() {
        parts.push(if ru {
            format!("начинается с «{fixed}{start}»")
        } else {
            format!("starts with “{fixed}{start}”")
        });
    }
    if !end.is_empty() {
        parts.push(if ru { format!("заканчивается на «{end}»") } else { format!("ends with “{end}”") });
    }
    parts.join(if ru { " и " } else { " and " })
}

/// Сколько символов в начале и в конце адреса — заказанные (для подсветки).
fn wish_marks(wish: Option<&Wish>, address: &str) -> (usize, usize) {
    let Some(w) = wish.filter(|w| wish_matches(w, address)) else { return (0, 0) };
    let start = wanted_body(w.net, &w.start);
    let head = if start.is_empty() { 0 } else { fixed_start(w.net).len() + start.len() };
    let tail = w.end.trim().len();
    if head + tail > address.len() {
        (address.len(), 0)
    } else {
        (head, tail)
    }
}

/// Адрес моноширинным шрифтом: заказанные начало и конец — цветом акцента.
fn address_job(address: &str, head: usize, tail: usize, size: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let fmt = |color| egui::TextFormat { font_id: egui::FontId::monospace(size), color, ..Default::default() };
    let mid_end = address.len() - tail;
    job.append(&address[..head], 0.0, fmt(ACCENT));
    job.append(&address[head..mid_end], 0.0, fmt(CODE_TEXT));
    job.append(&address[mid_end..], 0.0, fmt(ACCENT));
    job
}

// ---------------------------------------------------------------------------
// Проверки, которые программа показывает покупателю
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Ok,
    Warn,
    Fail,
}

struct Check {
    mark: Mark,
    text: String,
}

/// Итог шага: список проверок и вывод.
struct Outcome {
    checks: Vec<Check>,
    passed: bool,
    summary: String,
    /// Проверенный адрес и его сеть (когда проверка пройдена).
    found: Option<(Network, String)>,
}

impl Outcome {
    fn new() -> Self {
        Outcome { checks: Vec::new(), passed: true, summary: String::new(), found: None }
    }

    fn ok(&mut self, text: String) {
        self.checks.push(Check { mark: Mark::Ok, text });
    }

    fn warn(&mut self, text: String) {
        self.checks.push(Check { mark: Mark::Warn, text });
    }

    fn fail(&mut self, text: String) {
        self.checks.push(Check { mark: Mark::Fail, text });
        self.passed = false;
    }
}

fn decode_error_text(e: SplitError, tag: &str, ru: bool) -> String {
    match (e, ru) {
        (SplitError::BadFormat, true) => format!("Это не тот код: нужен код, который начинается с {tag}. Скопируйте его целиком."),
        (SplitError::BadFormat, false) => format!("Wrong code: you need the code that starts with {tag}. Copy it in full."),
        (SplitError::BadChecksum, true) => {
            "Код скопирован с ошибкой или не полностью (контрольная сумма в конце не сошлась). Скопируйте его заново.".into()
        }
        (SplitError::BadChecksum, false) => {
            "The code was copied with a mistake or not in full (the checksum at the end does not match). Copy it again.".into()
        }
        (e, ru) => e.text(ru).to_string(),
    }
}

struct Wallet {
    net: Network,
    address: String,
    secret: String,
    /// Сокращённые половины и ключ — для наглядного «как получился ключ».
    a_short: String,
    b_short: String,
    k_short: String,
}

impl Drop for Wallet {
    fn drop(&mut self) {
        self.secret.zeroize();
        self.a_short.zeroize();
        self.k_short.zeroize();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    How,
    Request,
    Verify,
    Wallet,
}

enum Msg {
    Ok(String),
    Err(String),
}

/// Кружок шага в боковой панели.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Badge {
    Info,
    Todo(u8),
    Current(u8),
    Done,
}

struct BuyerApp {
    ru: bool,
    tab: Tab,
    orders: Vec<vault::Order>,
    selected: usize,
    new_net: Network,
    new_start: String,
    new_end: String,
    create_msg: Option<Msg>,
    copy_msg: Option<Msg>,
    proof_input: String,
    verify: Option<Outcome>,
    key_input: String,
    assemble: Option<Outcome>,
    wallet: Option<Wallet>,
    wallet_msg: Option<Msg>,
    show_secret: bool,
    copied: Option<String>,
    /// Значок программы в верхней полосе окна (загружается при первом кадре).
    logo: Option<egui::TextureHandle>,
}

impl BuyerApp {
    fn new() -> Self {
        let orders = vault::list();
        BuyerApp {
            ru: true,
            // Первый запуск — сначала объяснение.
            tab: if orders.is_empty() { Tab::How } else { Tab::Request },
            orders,
            selected: 0,
            new_net: Network::Tron,
            new_start: String::new(),
            new_end: String::new(),
            create_msg: None,
            copy_msg: None,
            proof_input: String::new(),
            verify: None,
            key_input: String::new(),
            assemble: None,
            wallet: None,
            wallet_msg: None,
            show_secret: false,
            copied: None,
            logo: None,
        }
    }

    fn t(&self, ru: &'static str, en: &'static str) -> &'static str {
        if self.ru {
            ru
        } else {
            en
        }
    }

    fn reload(&mut self, select_id: Option<&str>) {
        self.orders = vault::list();
        self.selected = select_id
            .and_then(|id| self.orders.iter().position(|o| o.id == id))
            .unwrap_or(0);
    }

    fn order(&self) -> Option<&vault::Order> {
        self.orders.get(self.selected)
    }

    // ---------------------------------------------------------------- элементы

    /// Главная кнопка действия: зелёная, во всю ширину блока.
    fn primary(ui: &mut egui::Ui, text: &str) -> egui::Response {
        let width = ui.available_width();
        // Раскладка по центру — чтобы надпись стояла посередине кнопки.
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.add(
                egui::Button::new(RichText::new(text).color(ON_ACCENT).strong().size(15.5))
                    .fill(ACCENT)
                    .stroke(egui::Stroke::NONE)
                    .rounding(10.0)
                    .min_size(Vec2::new(width, 46.0)),
            )
        })
        .inner
    }

    /// Второстепенная кнопка: тёмная, с рамкой.
    fn secondary(ui: &mut egui::Ui, text: &str) -> egui::Response {
        ui.add(
            egui::Button::new(RichText::new(text).color(TEXT).size(14.5))
                .fill(BUTTON_BG)
                .stroke(egui::Stroke::new(1.0, INPUT_BORDER))
                .rounding(8.0)
                .min_size(Vec2::new(0.0, 38.0)),
        )
    }

    fn show_msg(ui: &mut egui::Ui, msg: &Option<Msg>) {
        match msg {
            Some(Msg::Ok(text)) => {
                ui.label(RichText::new(format!("✔  {text}")).color(OK_TEXT));
            }
            Some(Msg::Err(text)) => {
                ui.label(RichText::new(text).color(ERR_TEXT));
            }
            None => {}
        }
    }

    /// Блок-карточка: всё, что относится к одному действию, — в одной рамке.
    fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
        Self::card_colored(ui, CARD_BG, CARD_BORDER, add);
    }

    fn card_colored(ui: &mut egui::Ui, fill: Color32, border: Color32, add: impl FnOnce(&mut egui::Ui)) {
        egui::Frame::none()
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, border))
            .rounding(14.0)
            .inner_margin(egui::Margin::same(20.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                // Без выравнивания по ширине: в колонках egui иначе растягивает
                // одиночные слова строки по буквам.
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.spacing_mut().item_spacing.y = 12.0;
                    add(ui);
                });
            });
        ui.add_space(8.0);
    }

    /// Заголовок блока.
    fn title(ui: &mut egui::Ui, text: &str) {
        ui.label(RichText::new(text).strong().size(17.0).color(Color32::WHITE));
    }

    /// Пояснение: мельче и спокойнее основного текста, чтобы не заслонять главное.
    fn hint(ui: &mut egui::Ui, text: &str) {
        ui.label(RichText::new(text).size(13.5).color(HINT_TEXT));
    }

    /// Заголовок экрана и строка пояснения под ним.
    fn page_head(ui: &mut egui::Ui, title: &str, sub: &str) {
        ui.label(RichText::new(title).strong().size(24.0).color(Color32::WHITE));
        ui.add_space(-4.0);
        ui.label(RichText::new(sub).size(14.5).color(HINT_TEXT));
        ui.add_space(8.0);
    }

    /// Тёмная подложка для кода, адреса или ключа.
    fn code_frame(ui: &mut egui::Ui, border: Color32, add: impl FnOnce(&mut egui::Ui)) {
        egui::Frame::none()
            .fill(BG)
            .stroke(egui::Stroke::new(1.0, border))
            .rounding(10.0)
            .inner_margin(egui::Margin::symmetric(14.0, 12.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                add(ui);
            });
    }

    /// Адрес или ключ — моноширинным шрифтом в отдельном тёмном блоке.
    fn code_block(ui: &mut egui::Ui, text: &str, color: Color32) {
        Self::code_frame(ui, INPUT_BORDER, |ui| {
            ui.add(egui::Label::new(RichText::new(text).monospace().size(15.0).color(color)).wrap());
        });
    }

    /// Подпись над полем ввода.
    fn field_label(ui: &mut egui::Ui, text: &str) {
        ui.label(RichText::new(text).size(13.0).color(HINT_TEXT));
        ui.add_space(-6.0);
    }

    /// Плашка для важного: подкрашенный фон и цветной текст.
    fn callout(ui: &mut egui::Ui, color: Color32, fill: Color32, text: &str) {
        egui::Frame::none()
            .fill(fill)
            .rounding(10.0)
            .inner_margin(egui::Margin::symmetric(14.0, 12.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(text).size(14.0).color(color));
            });
    }

    fn bullet(ui: &mut egui::Ui, text: &str) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(RichText::new("•").color(ACCENT).strong());
            ui.label(RichText::new(text).color(TEXT));
        });
    }

    fn check_row(ui: &mut egui::Ui, c: &Check) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let (icon, color) = match c.mark {
                Mark::Ok => ("✔", ACCENT),
                Mark::Warn => ("⚠", WARN_TEXT),
                Mark::Fail => ("✖", ERR_TEXT),
            };
            ui.label(RichText::new(icon).color(color).strong().size(15.0));
            ui.label(RichText::new(&c.text).color(if c.mark == Mark::Fail { ERR_TEXT } else { TEXT }));
        });
    }

    /// Список проверок шага.
    fn show_checks(ui: &mut egui::Ui, outcome: &Outcome, title: &str) {
        Self::card(ui, |ui| {
            Self::title(ui, title);
            ui.spacing_mut().item_spacing.y = 8.0;
            for c in &outcome.checks {
                Self::check_row(ui, c);
            }
        });
    }

    /// Итог проверки, которая не прошла: красный блок с выводом.
    fn show_failure(ui: &mut egui::Ui, summary: &str) {
        Self::card_colored(ui, ERR_BG, ERR_BORDER, |ui| {
            ui.label(RichText::new(format!("✖  {summary}")).strong().size(16.5).color(ERR_TEXT));
        });
    }

    /// Кружок шага: номер, галочка или «i».
    fn paint_badge(painter: &egui::Painter, center: egui::Pos2, badge: Badge) {
        use egui::{Align2, FontId, Stroke};
        let r = 14.0;
        let (fill, stroke, text, color) = match badge {
            Badge::Info => (Color32::TRANSPARENT, INPUT_BORDER, "i".to_string(), HINT_TEXT),
            Badge::Todo(n) => (Color32::TRANSPARENT, INPUT_BORDER, n.to_string(), HINT_TEXT),
            Badge::Current(n) => (ACCENT, ACCENT, n.to_string(), ON_ACCENT),
            Badge::Done => (Color32::from_rgb(30, 58, 42), Color32::from_rgb(30, 58, 42), "✔".to_string(), ACCENT),
        };
        painter.circle(center, r, fill, Stroke::new(1.0, stroke));
        painter.text(center, Align2::CENTER_CENTER, text, FontId::proportional(13.5), color);
    }

    /// Пункт боковой панели: кружок, название и состояние шага.
    fn nav_item(&mut self, ui: &mut egui::Ui, tab: Tab, badge: Badge, title: &str, sub: &str, compact: bool) {
        use egui::{Align2, FontId, Pos2};
        let height = if sub.is_empty() || compact { 48.0 } else { 58.0 };
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), egui::Sense::click());
        let active = self.tab == tab;
        let painter = ui.painter();
        if active {
            painter.rect_filled(rect, 10.0, NAV_ACTIVE);
        } else if resp.hovered() {
            painter.rect_filled(rect, 10.0, Color32::from_rgb(22, 28, 36));
        }
        if compact {
            Self::paint_badge(painter, rect.center(), badge);
        } else {
            let c = Pos2::new(rect.left() + 26.0, rect.center().y);
            Self::paint_badge(painter, c, badge);
            let x = rect.left() + 52.0;
            if sub.is_empty() {
                painter.text(Pos2::new(x, c.y), Align2::LEFT_CENTER, title, FontId::proportional(15.0), TEXT);
            } else {
                painter.text(Pos2::new(x, c.y - 9.0), Align2::LEFT_CENTER, title, FontId::proportional(15.0), TEXT);
                let sub_color = if badge == Badge::Done { OK_TEXT } else { HINT_TEXT };
                painter.text(Pos2::new(x, c.y + 10.0), Align2::LEFT_CENTER, sub, FontId::proportional(12.5), sub_color);
            }
        }
        let resp = if compact { resp.on_hover_text(title) } else { resp };
        if resp.clicked() {
            self.tab = tab;
        }
    }

    /// Боковая панель: «Как это работает» и три шага с их состоянием.
    fn sidebar(&mut self, ui: &mut egui::Ui, compact: bool) {
        let (made, verified, assembled) = match self.order() {
            Some(o) => (true, o.verified_proof.is_some(), o.assembled_address.is_some()),
            None => (false, false, false),
        };
        let badge = |tab: Tab, n: u8, done: bool, current: Tab| {
            if done {
                Badge::Done
            } else if tab == current {
                Badge::Current(n)
            } else {
                Badge::Todo(n)
            }
        };
        let cur = self.tab;
        ui.spacing_mut().item_spacing.y = 4.0;
        self.nav_item(ui, Tab::How, Badge::Info, self.t("Как это работает", "How it works"), "", compact);
        ui.add_space(4.0);
        let (x0, x1) = (ui.max_rect().left() + 6.0, ui.max_rect().right() - 6.0);
        let y = ui.cursor().top();
        ui.painter().hline(x0..=x1, y, egui::Stroke::new(1.0, CARD_BORDER));
        ui.add_space(8.0);
        let sub1 = if made { self.t("Создана", "Created") } else { self.t("Сеть и красивый адрес", "Network and nice address") };
        let sub2 = if verified { self.t("Адрес подтверждён", "Address verified") } else { self.t("До оплаты · VGPROOF1", "Before paying · VGPROOF1") };
        let sub3 = if assembled { self.t("Собран и проверен", "Assembled and verified") } else { self.t("После оплаты · VGKEY1", "After paying · VGKEY1") };
        self.nav_item(ui, Tab::Request, badge(Tab::Request, 1, made, cur), self.t("Заявка", "Request"), sub1, compact);
        self.nav_item(ui, Tab::Verify, badge(Tab::Verify, 2, verified, cur), self.t("Проверка адреса", "Verify address"), sub2, compact);
        self.nav_item(ui, Tab::Wallet, badge(Tab::Wallet, 3, assembled, cur), self.t("Кошелёк", "Wallet"), sub3, compact);
        if compact {
            return;
        }
        // Подсказка к текущему шагу — внизу панели: заголовок и текст парами.
        let notes: Vec<(&str, Color32, &str)> = match self.tab {
            Tab::How | Tab::Request => vec![
                (self.t("Можно отправлять", "Safe to send"), TEXT,
                    self.t("Код заявки VGREQ1 — это только «отпечаток».", "Request code VGREQ1 — it is only a “fingerprint”.")),
                (self.t("Никому не отправляйте", "Never send"), ERR_TEXT,
                    self.t("Файл заявки и готовый приватный ключ.", "The request file and the finished private key.")),
            ],
            Tab::Verify => vec![(self.t("Платите только после проверки", "Pay only after the check"), TEXT,
                self.t("Деньги на сам адрес не отправляйте, пока не соберёте кошелёк на шаге 3.",
                    "Do not send money to the address itself until you assemble the wallet in step 3."))],
            Tab::Wallet => vec![(self.t("Ключ — только у вас", "The key is only yours"), TEXT,
                self.t("Он собирается на этом компьютере. Продавец его не видел и не может получить.",
                    "It is assembled on this computer. The seller has never seen it and cannot get it."))],
        };
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            egui::Frame::none()
                .fill(CARD_BG)
                .stroke(egui::Stroke::new(1.0, CARD_BORDER))
                .rounding(12.0)
                .inner_margin(egui::Margin::same(14.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 4.0;
                    // Раскладка снизу вверх: строки добавляются в обратном порядке.
                    for (i, (head, color, text)) in notes.iter().enumerate().rev() {
                        ui.label(RichText::new(*text).size(12.5).color(HINT_TEXT));
                        ui.label(RichText::new(*head).strong().size(12.5).color(*color));
                        if i > 0 {
                            ui.add_space(6.0);
                        }
                    }
                });
        });
    }

    // ------------------------------------------------------------ как это работает

    fn how_tab(&mut self, ui: &mut egui::Ui) {
        Self::page_head(ui, self.t("Как это работает", "How it works"), self.t(
            "Красивый адрес кошелька, ключ от которого есть только у вас.",
            "A nice wallet address whose key only you have.",
        ));
        Self::card(ui, |ui| {
            Self::title(ui, self.t("Что вы получаете", "What you get"));
            ui.label(RichText::new(self.t(
                "Кошелёк с красивым адресом, например на «…Pet», ключ от которого есть только у вас. \
                 Продавец находит адрес, но не может воспользоваться кошельком.",
                "A wallet with a nice address, for example ending in “…Pet”, whose key only you have. \
                 The seller finds the address but cannot use the wallet.",
            )).color(TEXT));
        });

        Self::card(ui, |ui| {
            Self::title(ui, self.t("Как это устроено — по-простому", "How it works — the simple version"));
            ui.spacing_mut().item_spacing.y = 8.0;
            Self::bullet(ui, self.t(
                "Приватный ключ кошелька — это большое секретное число. У кого это число, тот и владеет кошельком.",
                "A wallet's private key is a big secret number. Whoever has that number owns the wallet.",
            ));
            Self::bullet(ui, self.t(
                "Здесь ключ складывается из двух половин: ваша половина + половина продавца = ключ кошелька.",
                "Here the key is made of two halves: your half + the seller's half = the wallet key.",
            ));
            Self::bullet(ui, self.t(
                "Вашу половину эта программа создаёт на вашем компьютере. Продавцу уходит только её «отпечаток» \
                 (публичная половина): по нему можно искать красивый адрес, но саму половину из него не получить.",
                "This program creates your half on your computer. The seller only gets its “fingerprint” \
                 (the public half): it is enough to search for a nice address, but the half itself cannot be recovered from it.",
            ));
            Self::bullet(ui, self.t(
                "Продавец подбирает свою половину так, чтобы адрес получился красивым. Ключа у него нет — для ключа \
                 нужна ваша половина, а её он никогда не видит.",
                "The seller picks their half so that the address looks nice. The seller has no key — the key needs \
                 your half, which the seller never sees.",
            ));
            Self::bullet(ui, self.t(
                "После оплаты продавец присылает свою половину, и программа складывает обе половины у вас.",
                "After payment the seller sends their half, and the program adds both halves on your computer.",
            ));
        });

        Self::card(ui, |ui| {
            Self::title(ui, self.t("Три шага", "Three steps"));
            for (n, what, sent) in [
                (1u8, self.t("Заявка: выбираете сеть и красивое начало/конец адреса", "Request: choose the network and the nice start/end"),
                    self.t("вы → продавцу: код VGREQ1", "you → seller: code VGREQ1")),
                (2, self.t("Проверка: до оплаты проверяете найденный адрес", "Verify: check the found address before paying"),
                    self.t("продавец → вам: код VGPROOF1", "seller → you: code VGPROOF1")),
                (3, self.t("Кошелёк: после оплаты собираете ключ", "Wallet: assemble the key after paying"),
                    self.t("продавец → вам: код VGKEY1", "seller → you: code VGKEY1")),
            ] {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::hover());
                    Self::paint_badge(ui.painter(), rect.center(), Badge::Todo(n));
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(RichText::new(what).color(TEXT));
                        ui.label(RichText::new(sent).size(13.0).monospace().color(HINT_TEXT));
                    });
                });
            }
        });

        Self::card(ui, |ui| {
            Self::title(ui, self.t("Что можно отправлять, а что нельзя", "What you may and may not send"));
            ui.spacing_mut().item_spacing.y = 8.0;
            Self::check_row(ui, &Check { mark: Mark::Ok, text: self.t(
                "Код заявки VGREQ1 — можно показывать кому угодно, это только «отпечаток».",
                "Request code VGREQ1 — safe to show anyone, it is only a “fingerprint”.").into() });
            Self::check_row(ui, &Check { mark: Mark::Fail, text: self.t(
                "Файл заявки (папка orders) — никому. В нём ваша секретная половина.",
                "The request file (orders folder) — never. It holds your secret half.").into() });
            Self::check_row(ui, &Check { mark: Mark::Fail, text: self.t(
                "Готовый приватный ключ — никому, в том числе продавцу.",
                "The finished private key — never, not even to the seller.").into() });
        });

        Self::card(ui, |ui| {
            Self::title(ui, self.t("Как самому убедиться, что всё честно", "How to make sure yourself that it is honest"));
            ui.spacing_mut().item_spacing.y = 8.0;
            Self::bullet(ui, self.t(
                "Программа работает без интернета: в ней нет сетевого кода. Можете отключить интернет — всё будет работать так же.",
                "The program works without internet: it has no network code. Turn the internet off — everything works the same.",
            ));
            Self::bullet(ui, self.t(
                "Ваша секретная половина создаётся генератором случайных чисел Windows, хранится только в файле \
                 заявки на этом компьютере и никогда не покидает его: ни продавцу, ни кому-либо ещё она не передаётся.",
                "Your secret half comes from the Windows random number generator, is stored only in the request \
                 file on this computer and never leaves it: it is never sent to the seller or anyone else.",
            ));
            Self::bullet(ui, self.t(
                "Программа ничему не верит на слово: на шагах 2 и 3 она сама пересчитывает адрес и показывает \
                 список каждой проверки. Подменённый или чужой код она обнаружит, поэтому обман с подменой \
                 адреса или ключа исключён.",
                "The program trusts nothing: in steps 2 and 3 it recomputes the address itself and lists every \
                 check. It detects a forged or foreign code, so cheating by swapping the address or the key is ruled out.",
            ));
            Self::bullet(ui, self.t(
                "Исходный код программы открыт и небольшой: вся математика — в одном файле split_key.rs. \
                 Программу можно собрать самому по пошаговой инструкции в файле README.md.",
                "The source code is open and small: all the math is in one file, split_key.rs. You can build \
                 the program yourself by following the step-by-step guide in README.md.",
            ));
        });
        Self::callout(ui, WARN_TEXT, WARN_BG, self.t(
            "Отправляйте деньги на адрес только после шага 3, когда ключ собран и импортирован в ваш кошелёк.",
            "Send money to the address only after step 3, when the key is assembled and imported into your wallet.",
        ));
        ui.add_space(12.0);
        if Self::primary(ui, self.t("Перейти к шагу 1", "Go to step 1")).clicked() {
            self.tab = Tab::Request;
        }
    }

    // ------------------------------------------------------------------ шаг 1

    fn request_tab(&mut self, ui: &mut egui::Ui) {
        Self::page_head(ui, self.t("Шаг 1 · Заявка", "Step 1 · Request"), self.t(
            "Опишите адрес, который хотите, и отправьте заявку продавцу.",
            "Describe the address you want and send the request to the seller.",
        ));
        if ui.available_width() >= 760.0 {
            ui.columns(2, |cols| {
                self.new_request_card(&mut cols[0]);
                self.send_card(&mut cols[1]);
            });
        } else {
            self.new_request_card(ui);
            self.send_card(ui);
        }
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!(
                "{} {}",
                self.t("Файлы заявок:", "Request files:"),
                vault::orders_dir().display()
            ))
            .size(12.0)
            .color(FAINT_TEXT),
        );
    }

    fn new_request_card(&mut self, ui: &mut egui::Ui) {
        Self::card(ui, |ui| {
            Self::title(ui, self.t("Новая заявка", "New request"));
            Self::hint(ui, self.t(
                "Укажите, какой адрес вы хотите: на шаге 2 программа проверит, что продавец нашёл именно его.",
                "Describe the address you want: in step 2 the program checks that the seller found exactly that.",
            ));
            Self::field_label(ui, self.t("Сеть", "Network"));
            egui::ComboBox::from_id_salt("wish_net")
                .selected_text(RichText::new(self.new_net.label()).color(TEXT))
                .width(ui.available_width())
                .height(420.0)
                .show_ui(ui, |ui| {
                    for net in Network::ALL {
                        ui.selectable_value(&mut self.new_net, net, net.label());
                    }
                });
            Self::field_label(ui, self.t("Начало адреса", "Address start"));
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                // Постоянное начало адреса сети — рядом с полем, отдельной меткой.
                egui::Frame::none()
                    .fill(BUTTON_BG)
                    .rounding(8.0)
                    .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new(fixed_start(self.new_net)).monospace().size(15.0).color(HINT_TEXT));
                    });
                let hint = self.t("например: Pet", "e.g. Pet");
                ui.add(input(&mut self.new_start, hint, 1));
            });
            Self::field_label(ui, self.t("Конец адреса · необязательно", "Address end · optional"));
            let hint = self.t("например: 777", "e.g. 777");
            ui.add(input(&mut self.new_end, hint, 1));

            // Предпросмотр: как будет выглядеть адрес.
            let net = self.new_net;
            let start = wanted_body(net, &self.new_start).to_string();
            let end = self.new_end.trim().to_string();
            let head = format!("{}{start}", fixed_start(net));
            let dots = typical_len(net).saturating_sub(head.len() + end.len()).max(3);
            egui::Frame::none()
                .fill(INPUT_BG)
                .stroke(egui::Stroke::new(1.0, CARD_BORDER))
                .rounding(10.0)
                .inner_margin(egui::Margin::symmetric(14.0, 12.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.label(RichText::new(self.t("Так будет выглядеть адрес", "This is how the address will look")).size(12.5).color(HINT_TEXT));
                    let mut job = egui::text::LayoutJob::default();
                    let fmt = |color| egui::TextFormat { font_id: egui::FontId::monospace(16.0), color, ..Default::default() };
                    job.append(&head, 0.0, fmt(ACCENT));
                    job.append(&"•".repeat(dots), 0.0, fmt(FAINT_TEXT));
                    job.append(&end, 0.0, fmt(ACCENT));
                    ui.add(egui::Label::new(job).wrap());
                });
            Self::callout(ui, WARN_TEXT, WARN_BG, self.t(
                "Файл заявки хранит вашу секретную половину ключа: никому его не передавайте и не удаляйте до \
                 получения кошелька — без него собрать заказанный кошелёк будет невозможно.",
                "The request file holds your secret key half: never share it and do not delete it until you \
                 receive the wallet — without it the ordered wallet cannot be assembled.",
            ));
            if Self::primary(ui, self.t("Создать заявку", "Create request")).clicked() {
                self.create_request();
            }
            Self::show_msg(ui, &self.create_msg);
        });
    }

    fn send_card(&mut self, ui: &mut egui::Ui) {
        let ru = self.ru;
        let Some(order) = self.order().cloned() else {
            Self::card(ui, |ui| {
                Self::title(ui, self.t("Что отправить продавцу", "What to send the seller"));
                Self::hint(ui, self.t(
                    "Здесь появится сообщение для продавца, когда вы создадите заявку.",
                    "The message for the seller appears here once you create a request.",
                ));
            });
            return;
        };
        Self::card_colored(ui, CARD_BG, OK_BORDER, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::hover());
                Self::paint_badge(ui.painter(), rect.center(), Badge::Done);
                Self::title(ui, &format!("{} {} · {}", self.t("Заявка", "Request"), order.id, date(order.created)));
            });
            Self::hint(ui, self.t(
                "Скопируйте сообщение и отправьте его продавцу в Telegram-боте. В нём сеть, ваш заказ и код заявки — секрета там нет.",
                "Copy the message and send it to the seller in the Telegram bot. It contains the network, your order and the request code — no secret.",
            ));
            let message = seller_message(&order, ru);
            Self::code_frame(ui, CARD_BORDER, |ui| {
                ui.add(egui::Label::new(RichText::new(&message).monospace().size(13.5).color(CODE_TEXT)).wrap());
            });
            // Код заявки и сообщение публичные — обычное копирование.
            if Self::primary(ui, self.t("Копировать сообщение", "Copy message")).clicked() {
                ui.ctx().copy_text(message.clone());
                self.copy_msg = Some(Msg::Ok(self.t("Сообщение скопировано.", "Message copied.").into()));
            }
            Self::show_msg(ui, &self.copy_msg);
            // Подробности — свёрнуты, чтобы не отвлекать от главного.
            egui::CollapsingHeader::new(
                RichText::new(self.t("Что внутри кода заявки", "What is inside the request code")).color(HINT_TEXT),
            )
            .id_salt("req_parts_header")
            .default_open(false)
            .show(ui, |ui| {
                if let Ok(req) = Request::decode(&order.request) {
                    let pubhex = split_key::hex(&req.a_pub.serialize());
                    egui::Grid::new("req_parts").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                        ui.label(RichText::new("VGREQ1").monospace().color(TEXT));
                        ui.label(RichText::new(self.t("метка: это код заявки", "label: this is a request code")).color(HINT_TEXT));
                        ui.end_row();
                        ui.label(RichText::new(short(&pubhex)).monospace().color(TEXT));
                        ui.label(RichText::new(self.t("ваша публичная половина — «отпечаток», секрета в нём нет",
                            "your public half — a “fingerprint”, no secret in it")).color(HINT_TEXT));
                        ui.end_row();
                        ui.label(RichText::new(order.request.rsplit(':').next().unwrap_or("")).monospace().color(TEXT));
                        ui.label(RichText::new(self.t("контрольная сумма — защита от ошибок при копировании",
                            "checksum — protection against copy mistakes")).color(HINT_TEXT));
                        ui.end_row();
                        ui.label(RichText::new(&order.id).monospace().color(TEXT));
                        ui.label(RichText::new(self.t("номер заявки — по нему коды продавца связываются с этой заявкой",
                            "request number — links the seller's codes to this request")).color(HINT_TEXT));
                        ui.end_row();
                    });
                }
            });
        });
    }

    fn create_request(&mut self) {
        let ru = self.ru;
        let net = self.new_net;
        let start = wanted_body(net, &self.new_start).to_string();
        let end = self.new_end.trim().to_string();
        let msg = if start.is_empty() && end.is_empty() {
            Msg::Err(self.t("Укажите начало или конец адреса.", "Enter the address start or end.").into())
        } else if !charset_ok(net, &start) || !charset_ok(net, &end) {
            Msg::Err(allowed_chars_hint(net, ru).into())
        } else {
            match vault::create(&Wish { net, start, end }) {
                Ok(order) => {
                    self.reload(Some(&order.id));
                    self.new_start.clear();
                    self.new_end.clear();
                    self.copy_msg = None;
                    Msg::Ok(format!(
                        "{} {}. {}",
                        self.t("Заявка создана:", "Request created:"),
                        order.id,
                        self.t("Отправьте продавцу сообщение.", "Send the seller the message.")
                    ))
                }
                Err(e) => Msg::Err(e.text(ru)),
            }
        };
        self.create_msg = Some(msg);
    }

    // ------------------------------------------------------------------ шаг 2

    fn verify_tab(&mut self, ui: &mut egui::Ui) {
        Self::page_head(ui, self.t("Шаг 2 · Проверка адреса до оплаты", "Step 2 · Check the address before paying"), self.t(
            "Вставьте код от продавца — программа сама проверит, что адрес собран из вашей половины и подходит под заказ.",
            "Paste the seller's code — the program checks by itself that the address comes from your half and matches your order.",
        ));
        let mut clicked = false;
        Self::card(ui, |ui| {
            Self::field_label(ui, self.t("Код подтверждения от продавца (начинается с VGPROOF1)",
                "Confirmation code from the seller (starts with VGPROOF1)"));
            ui.add(input(&mut self.proof_input, "VGPROOF1:…", 3));
            clicked = Self::primary(ui, self.t("Проверить адрес", "Verify address")).clicked();
        });
        if clicked {
            self.verify_proof();
        }
        let wish = self.order().and_then(|o| o.wish.clone());
        let mut go_wallet = false;
        if let Some(outcome) = &self.verify {
            match &outcome.found {
                Some((net, address)) if outcome.passed => {
                    Self::card_colored(ui, OK_BG, OK_BORDER, |ui| {
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 17.0, ACCENT);
                            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "✔", egui::FontId::proportional(16.0), ON_ACCENT);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.label(RichText::new(self.t("Адрес настоящий — можно оплачивать", "The address is genuine — you can pay"))
                                    .strong().size(18.0).color(Color32::WHITE));
                                ui.label(RichText::new(&outcome.summary).size(13.5).color(Color32::from_rgb(169, 221, 190)));
                            });
                        });
                        Self::code_frame(ui, Color32::from_rgb(35, 64, 47), |ui| {
                            ui.label(RichText::new(format!("{} · {}", self.t("Найденный адрес", "Found address"), net.label()))
                                .size(12.5).color(HINT_TEXT));
                            let (head, tail) = wish_marks(wish.as_ref(), address);
                            ui.add(egui::Label::new(address_job(address, head, tail, 20.0)).wrap());
                        });
                    });
                }
                _ => Self::show_failure(ui, &outcome.summary),
            }
            Self::show_checks(ui, outcome, self.t("Что проверила программа", "What the program checked"));
            if outcome.passed {
                Self::card(ui, |ui| {
                    Self::hint(ui, self.t(
                        "После оплаты продавец пришлёт код VGKEY1 — с ним вы соберёте кошелёк.",
                        "After payment the seller sends the VGKEY1 code — you assemble the wallet with it.",
                    ));
                    go_wallet = Self::secondary(ui, self.t("Перейти к шагу 3  →", "Go to step 3  →")).clicked();
                });
            }
        }
        if go_wallet {
            self.tab = Tab::Wallet;
        }
    }

    fn find_order(&mut self, id: &str) -> Option<vault::Order> {
        let pos = self.orders.iter().position(|o| o.id == id)?;
        self.selected = pos;
        Some(self.orders[pos].clone())
    }

    /// Сеть и шаблон из кода против того, что покупатель заказал.
    fn check_wish(&self, out: &mut Outcome, order: &vault::Order, net: Network, address: &str) {
        let ru = self.ru;
        let Some(wish) = &order.wish else { return };
        if wish.net == net {
            out.ok(format!("{} {} — {}", self.t("Сеть:", "Network:"), net.label(),
                self.t("как вы заказывали.", "as you ordered.")));
        } else {
            out.fail(format!("{} {}, {} {}.", self.t("В коде сеть", "The code has network"), net.label(),
                self.t("а вы заказывали", "but you ordered"), wish.net.label()));
        }
        if wish_matches(wish, address) {
            out.ok(format!("{} {address} {}: {}.", self.t("Адрес", "Address"),
                self.t("подходит под заказ", "matches your order"), wish_text(wish, ru)));
        } else {
            out.fail(format!("{} {address} {}: {}.", self.t("Адрес", "Address"),
                self.t("НЕ подходит под заказ, вы хотели", "does NOT match your order, you wanted"), wish_text(wish, ru)));
        }
    }

    fn verify_proof(&mut self) {
        let ru = self.ru;
        let mut out = Outcome::new();
        'checks: {
            let proof = match Proof::decode(&self.proof_input) {
                Ok(p) => p,
                Err(e) => {
                    out.fail(decode_error_text(e, "VGPROOF1", ru));
                    break 'checks;
                }
            };
            out.ok(self.t("Код цел: контрольная сумма сошлась, он скопирован без ошибок.",
                "The code is intact: the checksum matches, it was copied without mistakes.").into());
            let Some(order) = self.find_order(&proof.order_id) else {
                out.fail(format!("{} {} {}", self.t("Код относится к заявке", "The code belongs to request"), proof.order_id,
                    self.t("— её нет на этом компьютере.", "— it is not on this computer.")));
                break 'checks;
            };
            out.ok(format!("{} {}.", self.t("Код относится к вашей заявке", "The code belongs to your request"), order.id));
            self.check_wish(&mut out, &order, proof.net, &proof.address);
            let Ok(request) = Request::decode(&order.request) else {
                out.fail(self.t("Файл заявки повреждён.", "The request file is damaged.").into());
                break 'checks;
            };
            match proof.verify(&request) {
                Ok(()) => out.ok(self.t(
                    "Адрес собран из вашей половины: программа сама сложила вашу публичную половину с публичной \
                     половиной продавца и получила именно этот адрес.",
                    "The address comes from your half: the program itself added your public half to the seller's \
                     public half and got exactly this address.",
                ).into()),
                Err(_) => out.fail(self.t(
                    "Адрес НЕ собран из вашей половины — продавец прислал чужой адрес.",
                    "The address does NOT come from your half — the seller sent a foreign address.",
                ).into()),
            }
            if out.passed {
                if let Err(e) = vault::save_verified_proof(&order, &proof.encode()) {
                    out.fail(e.text(ru));
                }
                self.reload(Some(&order.id));
                out.found = Some((proof.net, proof.address.clone()));
            }
        }
        out.summary = if out.passed {
            self.t(
                "Всё сходится. Адрес настоящий, и управлять им сможете только вы: продавцу не хватает вашей \
                 секретной половины. Можно оплачивать заказ. Деньги на сам адрес не отправляйте до шага 3.",
                "Everything matches. The address is genuine and only you will control it: the seller lacks your \
                 secret half. You can pay for the order. Do not send money to the address itself before step 3.",
            )
        } else {
            self.t(
                "Проверка не пройдена — не оплачивайте и напишите продавцу.",
                "The check failed — do not pay, contact the seller.",
            )
        }
        .to_string();
        self.verify = Some(out);
    }

    // ------------------------------------------------------------------ шаг 3

    fn wallet_tab(&mut self, ui: &mut egui::Ui) {
        Self::page_head(ui, self.t("Шаг 3 · Кошелёк", "Step 3 · Wallet"), self.t(
            "После оплаты вставьте половину ключа от продавца — программа соберёт ключ и проверит его.",
            "After paying, paste the seller's key half — the program assembles the key and checks it.",
        ));
        let mut clicked = false;
        Self::card(ui, |ui| {
            Self::field_label(ui, self.t("Половина ключа от продавца (начинается с VGKEY1)",
                "Key half from the seller (starts with VGKEY1)"));
            ui.add(input(&mut self.key_input, "VGKEY1:…", 3));
            clicked = Self::primary(ui, self.t("Собрать кошелёк", "Assemble wallet")).clicked();
            // Пройденные проверки свёрнуты в одну строку.
            if let Some(outcome) = self.assemble.as_ref().filter(|o| o.passed) {
                let head = format!("✔  {} {} — {}",
                    self.t("Все проверки пройдены:", "All checks passed:"), outcome.checks.len(),
                    self.t("показать", "show"));
                egui::CollapsingHeader::new(RichText::new(head).color(OK_TEXT))
                    .id_salt("assemble_checks")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        for c in &outcome.checks {
                            Self::check_row(ui, c);
                        }
                    });
            }
        });
        if clicked {
            self.assemble();
        }
        if let Some(outcome) = self.assemble.as_ref().filter(|o| !o.passed) {
            Self::show_failure(ui, &outcome.summary);
            Self::show_checks(ui, outcome, self.t("Что проверила программа", "What the program checked"));
        }

        let mut actions = WalletActions::default();
        if self.wallet.is_some() {
            if ui.available_width() >= 760.0 {
                ui.columns(2, |cols| {
                    self.wallet_card(&mut cols[0], &mut actions);
                    self.key_card(&mut cols[1], &mut actions);
                });
            } else {
                self.wallet_card(ui, &mut actions);
                self.key_card(ui, &mut actions);
            }
        }
        if let Some(address) = actions.copy_address.take() {
            ui.ctx().copy_text(address);
            self.wallet_msg = Some(Msg::Ok(self.t("Адрес скопирован.", "Address copied.").into()));
        }
        if let Some(mut secret) = actions.copy.take() {
            self.wallet_msg = Some(if copy_secret(&secret) {
                if let Some(mut old) = self.copied.replace(secret.clone()) {
                    old.zeroize();
                }
                Msg::Ok(self.t("Ключ скопирован. При выходе из программы он будет стёрт из буфера обмена.", "Key copied. It is wiped from the clipboard when the program exits.").into())
            } else {
                Msg::Err(self.t("Не удалось скопировать ключ.", "Could not copy the key.").into())
            });
            secret.zeroize();
        }
        if actions.toggle {
            self.show_secret = !self.show_secret;
        }
        if actions.save {
            self.save_wallet();
        }
        if actions.close {
            self.wallet = None;
            self.show_secret = false;
            self.wallet_msg = None;
        }
    }

    /// Готовый кошелёк: адрес и наглядное «как получился ключ».
    fn wallet_card(&self, ui: &mut egui::Ui, actions: &mut WalletActions) {
        let Some(w) = &self.wallet else { return };
        let wish = self.order().and_then(|o| o.wish.clone());
        Self::card_colored(ui, OK_BG, OK_BORDER, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(self.t("Ваш кошелёк готов", "Your wallet is ready")).strong().size(18.0).color(Color32::WHITE));
                egui::Frame::none()
                    .fill(BG)
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(35, 64, 47)))
                    .rounding(20.0)
                    .inner_margin(egui::Margin::symmetric(10.0, 3.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new(w.net.label()).size(12.5).color(Color32::from_rgb(169, 221, 190)));
                    });
            });
            Self::code_frame(ui, Color32::from_rgb(35, 64, 47), |ui| {
                ui.label(RichText::new(self.t("Адрес", "Address")).size(12.5).color(HINT_TEXT));
                let (head, tail) = wish_marks(wish.as_ref(), &w.address);
                ui.add(egui::Label::new(address_job(&w.address, head, tail, 18.0)).wrap());
                if Self::secondary(ui, self.t("Копировать адрес", "Copy address")).clicked() {
                    actions.copy_address = Some(w.address.clone());
                }
            });
            ui.label(RichText::new(self.t("Как получился ключ", "How the key was made")).size(12.5).color(HINT_TEXT));
            Self::code_frame(ui, Color32::from_rgb(35, 64, 47), |ui| {
                egui::Grid::new("sum_grid").num_columns(3).spacing([10.0, 8.0]).show(ui, |ui| {
                    for (sign, value, label, accent) in [
                        ("", &w.a_short, self.t("ваша половина", "your half"), false),
                        ("+", &w.b_short, self.t("половина продавца", "seller's half"), false),
                        ("=", &w.k_short, self.t("ключ кошелька", "wallet key"), true),
                    ] {
                        ui.label(RichText::new(sign).monospace().size(14.0).color(HINT_TEXT));
                        ui.label(RichText::new(value).monospace().size(13.5).color(if accent { ACCENT } else { CODE_TEXT }));
                        ui.label(RichText::new(label).size(12.5).color(HINT_TEXT));
                        ui.end_row();
                    }
                });
            });
        });
    }

    /// Приватный ключ (скрыт, пока его не показать) и что делать дальше.
    fn key_card(&self, ui: &mut egui::Ui, actions: &mut WalletActions) {
        let Some(w) = &self.wallet else { return };
        Self::card(ui, |ui| {
            Self::title(ui, self.t("Приватный ключ", "Private key"));
            let shown = if self.show_secret { w.secret.clone() } else { "•".repeat(32) };
            Self::code_block(ui, &shown, if self.show_secret { TEXT } else { FAINT_TEXT });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                let toggle = if self.show_secret { self.t("Скрыть", "Hide") } else { self.t("Показать", "Show") };
                if Self::secondary(ui, toggle).clicked() {
                    actions.toggle = true;
                }
                if Self::secondary(ui, self.t("Копировать ключ", "Copy key")).clicked() {
                    actions.copy = Some(w.secret.clone());
                }
                if Self::secondary(ui, self.t("Сохранить в файл", "Save to file")).clicked() {
                    actions.save = true;
                }
                if Self::secondary(ui, self.t("Закрыть", "Close")).clicked() {
                    actions.close = true;
                }
            });
            Self::show_msg(ui, &self.wallet_msg);
            Self::callout(ui, ERR_TEXT, ERR_BG, self.t(
                "Ключ даёт полный доступ к кошельку. Никому его не показывайте и не отправляйте, \
                 в том числе продавцу.",
                "The key gives full access to the wallet. Never show or send it to anyone, \
                 including the seller.",
            ));
        });
        Self::card(ui, |ui| {
            Self::title(ui, self.t("Что дальше", "What next"));
            ui.spacing_mut().item_spacing.y = 8.0;
            Self::bullet(ui, &if self.ru {
                format!("Импортируйте ключ в кошелёк и убедитесь, что он показывает адрес {}.", w.address)
            } else {
                format!("Import the key into a wallet and make sure it shows the address {}.", w.address)
            });
            Self::bullet(ui, self.t(
                "Сохраните ключ в надёжном месте (на бумаге или в менеджере паролей).",
                "Store the key safely (on paper or in a password manager).",
            ));
            Self::bullet(ui, self.t(
                "Файл заявки и код VGKEY1 после этого лучше удалить: вместе они дают ключ.",
                "Afterwards it is best to delete the request file and the VGKEY1 code: together they give the key.",
            ));
        });
    }

    fn assemble(&mut self) {
        let ru = self.ru;
        self.wallet = None;
        self.wallet_msg = None;
        self.show_secret = false;
        let mut out = Outcome::new();
        'checks: {
            let half = match KeyHalf::decode(&self.key_input) {
                Ok(h) => h,
                Err(e) => {
                    out.fail(decode_error_text(e, "VGKEY1", ru));
                    break 'checks;
                }
            };
            out.ok(self.t("Код цел: контрольная сумма сошлась, он скопирован без ошибок.",
                "The code is intact: the checksum matches, it was copied without mistakes.").into());
            let Some(order) = self.find_order(&half.order_id) else {
                out.fail(format!("{} {} {}", self.t("Код относится к заявке", "The code belongs to request"), half.order_id,
                    self.t("— её нет на этом компьютере.", "— it is not on this computer.")));
                break 'checks;
            };
            out.ok(format!("{} {}.", self.t("Код относится к вашей заявке", "The code belongs to your request"), order.id));
            let verified = order.verified_proof.as_deref().and_then(|c| Proof::decode(c).ok());
            match &verified {
                Some(p) if *p == half.proof() => out.ok(self.t(
                    "Половина продавца та самая, что вы проверили на шаге 2 (её «отпечаток» совпал).",
                    "The seller's half is exactly the one you verified in step 2 (its “fingerprint” matches).",
                ).into()),
                Some(_) => {
                    out.fail(self.t(
                        "Половина продавца НЕ совпадает с той, что вы проверили на шаге 2.",
                        "The seller's half does NOT match the one you verified in step 2.",
                    ).into());
                    break 'checks;
                }
                None => out.warn(self.t(
                    "Шаг 2 для этой заявки не выполнялся — адрес будет проверен сейчас.",
                    "Step 2 was skipped for this request — the address is checked now.",
                ).into()),
            }
            self.check_wish(&mut out, &order, half.net, &half.address);
            let Ok(request) = Request::decode(&order.request) else {
                out.fail(self.t("Файл заявки повреждён.", "The request file is damaged.").into());
                break 'checks;
            };
            if half.verify(&request).is_err() {
                out.fail(self.t(
                    "Адрес НЕ собирается из вашей половины и присланной половины продавца.",
                    "The address does NOT come from your half and the seller's half.",
                ).into());
                break 'checks;
            }
            out.ok(self.t(
                "Адрес собирается из вашей половины и половины продавца.",
                "The address comes from your half and the seller's half.",
            ).into());
            let mut a = match vault::secret(&order) {
                Ok(a) => a,
                Err(e) => {
                    out.fail(e.text(ru));
                    break 'checks;
                }
            };
            out.ok(self.t(
                "Ваша секретная половина прочитана из файла заявки на этом компьютере.",
                "Your secret half was read from the request file on this computer.",
            ).into());
            let combined = half.combine(&a, verified.as_ref());
            let mut a_hex = split_key::hex(&a.secret_bytes());
            a.non_secure_erase();
            let mut k = match combined {
                Ok(k) => k,
                Err(e) => {
                    a_hex.zeroize();
                    out.fail(e.text(ru).to_string());
                    break 'checks;
                }
            };
            out.ok(self.t(
                "Ваша половина + половина продавца = приватный ключ кошелька.",
                "Your half + the seller's half = the wallet private key.",
            ).into());
            out.ok(format!(
                "{} {}.",
                self.t("Повторная проверка уже из готового ключа: он даёт ровно адрес", "Re-check from the finished key: it gives exactly the address"),
                half.address
            ));
            let mut k_hex = split_key::hex(&k.secret_bytes());
            let wallet = Wallet {
                net: half.net,
                address: half.address.clone(),
                secret: split_key::export_secret(half.net, &k),
                a_short: short(&a_hex),
                b_short: short(&split_key::hex(&half.b.secret_bytes())),
                k_short: short(&k_hex),
            };
            k.non_secure_erase();
            a_hex.zeroize();
            k_hex.zeroize();
            if let Err(e) = vault::save_assembled(&order, &half.address) {
                out.warn(e.text(ru));
            }
            self.reload(Some(&order.id));
            out.found = Some((half.net, half.address.clone()));
            self.wallet = Some(wallet);
        }
        out.summary = if out.passed {
            self.t(
                "Кошелёк собран и проверен. Ключ есть только у вас.",
                "The wallet is assembled and verified. Only you have the key.",
            )
        } else {
            self.t(
                "Кошелёк не собран — проверка не пройдена. Напишите продавцу.",
                "The wallet was not assembled — a check failed. Contact the seller.",
            )
        }
        .to_string();
        self.assemble = Some(out);
    }

    fn save_wallet(&mut self) {
        let Some(w) = &self.wallet else { return };
        let dir = vault::data_dir().join("wallets");
        let doc = serde_json::json!({
            "Network": w.net.label(),
            "Address": w.address,
            "PrivateKey": w.secret,
            "Note": "Полный приватный ключ. Храните файл в надёжном месте и никому его не передавайте. / \
                     Full private key. Keep this file safe and never share it.",
        });
        let result = std::fs::create_dir_all(&dir).and_then(|_| {
            let mut n = 0u32;
            loop {
                let name = if n == 0 { format!("{}.json", w.address) } else { format!("{}_{n}.json", w.address) };
                let path = dir.join(name);
                match OpenOptions::new().write(true).create_new(true).open(&path) {
                    Ok(mut f) => {
                        let mut text = serde_json::to_string_pretty(&doc).unwrap_or_default();
                        let r = f.write_all(text.as_bytes()).and_then(|_| f.sync_all());
                        text.zeroize();
                        return r.map(|_| path);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && n < 1000 => n += 1,
                    Err(e) => return Err(e),
                }
            }
        });
        self.wallet_msg = Some(match result {
            Ok(path) => Msg::Ok(format!("{} {}", self.t("Сохранено:", "Saved:"), path.display())),
            Err(e) => Msg::Err(format!("{} {e}", self.t("Не удалось сохранить:", "Could not save:"))),
        });
    }
}

/// Что нажато в блоках готового кошелька (выполняется после отрисовки).
#[derive(Default)]
struct WalletActions {
    toggle: bool,
    copy: Option<String>,
    copy_address: Option<String>,
    save: bool,
    close: bool,
}

impl Drop for WalletActions {
    fn drop(&mut self) {
        if let Some(s) = &mut self.copy {
            s.zeroize();
        }
    }
}

/// Готовое сообщение продавцу: сеть, заказ и код заявки.
fn seller_message(order: &vault::Order, ru: bool) -> String {
    let mut m = String::new();
    m.push_str(if ru { "Заказ vanity-кошелька (разделённый ключ)\n" } else { "Vanity wallet order (split key)\n" });
    if let Some(w) = &order.wish {
        let fixed = fixed_start(w.net);
        let start = wanted_body(w.net, &w.start);
        let dash = "—";
        m.push_str(&format!("{} {}\n", if ru { "Сеть:" } else { "Network:" }, w.net.label()));
        m.push_str(&format!(
            "{} {}\n",
            if ru { "Начало адреса:" } else { "Address start:" },
            if start.is_empty() { dash.to_string() } else { format!("{fixed}{start}") }
        ));
        m.push_str(&format!(
            "{} {}\n",
            if ru { "Конец адреса:" } else { "Address end:" },
            if w.end.trim().is_empty() { dash } else { w.end.trim() }
        ));
    }
    m.push_str(&format!("{} {}", if ru { "Код заявки:" } else { "Request code:" }, order.request));
    m
}

/// Кнопка окна (свернуть, развернуть, восстановить, закрыть).
#[derive(Clone, Copy, PartialEq, Eq)]
enum WinButton {
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// Кнопка управления окном с нарисованной иконкой.
fn window_button(ui: &mut egui::Ui, kind: WinButton) -> egui::Response {
    use egui::{Rect, Stroke};
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(40.0, 30.0), egui::Sense::click());
    let hovered = resp.hovered();
    let pressed = resp.is_pointer_button_down_on();
    let painter = ui.painter();
    if hovered {
        let fill = match (kind, pressed) {
            (WinButton::Close, false) => Color32::from_rgb(196, 64, 64),
            (WinButton::Close, true) => Color32::from_rgb(164, 48, 48),
            (_, true) => Color32::from_white_alpha(46),
            (_, false) => Color32::from_white_alpha(28),
        };
        painter.rect_filled(rect, 6.0, fill);
    }
    let color = if hovered && kind == WinButton::Close { Color32::WHITE } else { HINT_TEXT };
    let stroke = Stroke::new(1.5, color);
    let c = rect.center();
    match kind {
        WinButton::Minimize => {
            painter.line_segment([c + Vec2::new(-5.0, 0.5), c + Vec2::new(5.0, 0.5)], stroke);
        }
        WinButton::Maximize => {
            painter.rect_stroke(Rect::from_center_size(c, Vec2::splat(10.0)), 1.5, stroke);
        }
        WinButton::Restore => {
            painter.rect_stroke(Rect::from_center_size(c + Vec2::new(-1.5, 1.5), Vec2::splat(8.0)), 1.5, stroke);
            painter.line_segment([c + Vec2::new(-2.0, -4.5), c + Vec2::new(5.5, -4.5)], stroke);
            painter.line_segment([c + Vec2::new(5.5, -4.5), c + Vec2::new(5.5, 3.0)], stroke);
        }
        WinButton::Close => {
            painter.line_segment([c + Vec2::new(-5.0, -5.0), c + Vec2::new(5.0, 5.0)], stroke);
            painter.line_segment([c + Vec2::new(5.0, -5.0), c + Vec2::new(-5.0, 5.0)], stroke);
        }
    }
    resp
}

/// Край или угол окна под указателем — для растягивания окна без рамки.
/// Углы шире краёв, чтобы их было легко поймать.
fn resize_edge(ctx: &egui::Context) -> Option<egui::ResizeDirection> {
    use egui::ResizeDirection as D;
    ctx.input(|i| {
        let rect = i.screen_rect();
        let p = i.pointer.hover_pos()?;
        const EDGE: f32 = 5.0;
        const CORNER: f32 = 14.0;
        let near = |d: f32, w: f32| d >= -1.0 && d <= w;
        let (dl, dr, dt, db) = (p.x - rect.left(), rect.right() - p.x, p.y - rect.top(), rect.bottom() - p.y);
        let (l, r, t, b) = (near(dl, EDGE), near(dr, EDGE), near(dt, EDGE), near(db, EDGE));
        let (lc, rc, tc, bc) = (near(dl, CORNER), near(dr, CORNER), near(dt, CORNER), near(db, CORNER));
        Some(match () {
            _ if (l && tc) || (t && lc) => D::NorthWest,
            _ if (r && tc) || (t && rc) => D::NorthEast,
            _ if (l && bc) || (b && lc) => D::SouthWest,
            _ if (r && bc) || (b && rc) => D::SouthEast,
            _ if l => D::West,
            _ if r => D::East,
            _ if t => D::North,
            _ if b => D::South,
            _ => return None,
        })
    })
}

impl BuyerApp {
    /// Верхняя полоса окна без рамки Windows: значок и название, отметка
    /// «без интернета», язык, место для перетаскивания (двойной щелчок —
    /// развернуть/восстановить) и кнопки окна.
    fn title_bar(&mut self, ui: &mut egui::Ui, maximized: bool, wide: bool) {
        let ctx = ui.ctx().clone();
        let logo = self
            .logo
            .get_or_insert_with(|| {
                ctx.load_texture(
                    "app_icon",
                    egui::ColorImage::from_rgba_unmultiplied([256, 256], ICON_RGBA),
                    egui::TextureOptions::LINEAR,
                )
            })
            .clone();
        egui::Frame::none()
            .fill(SIDEBAR_BG)
            .inner_margin(egui::Margin { left: 16.0, right: 8.0, top: 8.0, bottom: 8.0 })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    ui.add(egui::Image::new(&logo).fit_to_exact_size(Vec2::splat(30.0)));
                    ui.label(RichText::new("Vanity Buyer").strong().size(16.0).color(Color32::WHITE));
                    if wide {
                        egui::Frame::none()
                            .fill(Color32::from_rgb(18, 36, 26))
                            .rounding(20.0)
                            .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                let (dot, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                                ui.painter().circle_filled(dot.center(), 4.0, ACCENT);
                                ui.label(RichText::new(self.t("Работает без интернета", "Works offline"))
                                    .size(12.5).color(OK_TEXT));
                            });
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        if window_button(ui, WinButton::Close).on_hover_text(self.t("Закрыть", "Close")).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        let (kind, tip) = if maximized {
                            (WinButton::Restore, self.t("Восстановить", "Restore"))
                        } else {
                            (WinButton::Maximize, self.t("Развернуть", "Maximize"))
                        };
                        if window_button(ui, kind).on_hover_text(tip).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }
                        if window_button(ui, WinButton::Minimize).on_hover_text(self.t("Свернуть", "Minimize")).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        ui.add_space(12.0);
                        self.language_switch(ui);
                        let drag = ui.allocate_response(
                            Vec2::new(ui.available_width().max(20.0), 32.0),
                            egui::Sense::click_and_drag(),
                        );
                        if drag.double_clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        } else if drag.drag_started_by(egui::PointerButton::Primary) {
                            // Перетаскивание делает сама Windows: развёрнутое окно при этом
                            // восстанавливается, как у обычных окон.
                            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                    });
                });
            });
    }

    /// Переключатель языка RU / EN.
    fn language_switch(&mut self, ui: &mut egui::Ui) {
        let mut set = None;
        egui::Frame::none()
            .fill(BUTTON_BG)
            .stroke(egui::Stroke::new(1.0, CARD_BORDER))
            .rounding(10.0)
            .inner_margin(egui::Margin::same(3.0))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                // Справа налево: сначала EN, потом RU.
                for (label, ru) in [("EN", false), ("RU", true)] {
                    let on = self.ru == ru;
                    let b = egui::Button::new(RichText::new(label).strong().size(12.5).color(if on { TEXT } else { HINT_TEXT }))
                        .fill(if on { Color32::from_rgb(42, 52, 64) } else { Color32::TRANSPARENT })
                        .stroke(egui::Stroke::NONE)
                        .rounding(7.0)
                        .min_size(Vec2::new(40.0, 28.0));
                    if ui.add(b).clicked() && !on {
                        set = Some(ru);
                    }
                }
            });
        if let Some(ru) = set {
            self.ru = ru;
            self.create_msg = None;
            self.copy_msg = None;
            self.verify = None;
            self.assemble = None;
            self.wallet_msg = None;
        }
    }
}

impl eframe::App for BuyerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Состояние «развёрнуто» — от самой Windows, а не своё: оно верно и
        // после Win+↑, двойного щелчка или перетаскивания развёрнутого окна.
        let maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
        // Растягивание за края — только у обычного (не развёрнутого) окна.
        let edge = if maximized { None } else { resize_edge(ctx) };
        // Развёрнутое окно без рамки Windows ставит за края экрана (на ширину
        // скрытой рамки, обычно 8 px) — на столько же отступаем внутрь, чтобы
        // кнопки окна и края интерфейса не обрезались.
        let inset = if maximized {
            ctx.input(|i| {
                let v = i.viewport();
                match (v.outer_rect, v.monitor_size) {
                    (Some(r), Some(m)) => ((r.width() - m.x) / 2.0).clamp(0.0, 16.0),
                    _ => 0.0,
                }
            })
        } else {
            0.0
        };
        let frame = egui::Frame::none()
            .fill(BG)
            .outer_margin(egui::Margin::same(inset))
            .inner_margin(egui::Margin::same(1.0))
            .stroke(if maximized { egui::Stroke::NONE } else { egui::Stroke::new(1.0, CARD_BORDER) });
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            let wide = ui.available_width() >= WIDE;
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            self.title_bar(ui, maximized, wide);
            let line = ui.cursor().top();
            ui.painter().hline(ui.max_rect().x_range(), line, egui::Stroke::new(1.0, CARD_BORDER));
            let body = ui.available_rect_before_wrap();
            let nav_w = if wide { 260.0 } else { 72.0 };
            let nav_rect = egui::Rect::from_min_size(body.min, Vec2::new(nav_w, body.height()));
            let main_rect = egui::Rect::from_min_max(egui::pos2(nav_rect.right(), body.top()), body.max);
            ui.painter().rect_filled(nav_rect, 0.0, SIDEBAR_BG);
            ui.painter().vline(nav_rect.right(), body.y_range(), egui::Stroke::new(1.0, CARD_BORDER));

            let mut nav_ui = ui.new_child(
                egui::UiBuilder::new().max_rect(nav_rect.shrink2(Vec2::new(12.0, 16.0))).layout(egui::Layout::top_down(egui::Align::Min)),
            );
            nav_ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
            self.sidebar(&mut nav_ui, !wide);

            let mut main_ui = ui.new_child(egui::UiBuilder::new().max_rect(main_rect).layout(egui::Layout::top_down(egui::Align::Min)));
            main_ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(&mut main_ui, |ui| {
                egui::Frame::none()
                    .inner_margin(egui::Margin { left: 28.0, right: 28.0, top: 24.0, bottom: 28.0 })
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        match self.tab {
                            Tab::How => self.how_tab(ui),
                            Tab::Request => self.request_tab(ui),
                            Tab::Verify => self.verify_tab(ui),
                            Tab::Wallet => self.wallet_tab(ui),
                        }
                    });
            });
            ui.allocate_rect(body, egui::Sense::hover());
        });
        if let Some(d) = edge {
            use egui::{CursorIcon as C, ResizeDirection as D};
            ctx.set_cursor_icon(match d {
                D::West | D::East => C::ResizeHorizontal,
                D::North | D::South => C::ResizeVertical,
                D::NorthWest | D::SouthEast => C::ResizeNwSe,
                D::NorthEast | D::SouthWest => C::ResizeNeSw,
            });
            if ctx.input(|i| i.pointer.primary_pressed()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(d));
            }
        }
        // Ссылок в программе нет; никакой URL не может быть передан браузеру.
        ctx.output_mut(|o| o.open_url = None);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(mut key) = self.copied.take() {
            clear_secret(&key);
            key.zeroize();
        }
        self.wallet = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(date(0), "01.01.1970");
        assert_eq!(date(951_782_400), "29.02.2000");
        assert_eq!(date(1_793_313_266), "29.10.2026");
    }

    /// Проверка заказа понимает шаблон так же, как программа продавца:
    /// постоянное начало сети можно вводить или нет.
    #[test]
    fn wish_matching() {
        let w = |net, start: &str, end: &str| Wish { net, start: start.into(), end: end.into() };
        let tron = "TH3PWxLoDjwGSiceCVmPGGgiCnbZqbpPet";
        assert!(wish_matches(&w(Network::Tron, "H3PW", ""), tron));
        assert!(wish_matches(&w(Network::Tron, "TH3PW", ""), tron));
        assert!(wish_matches(&w(Network::Tron, "", "Pet"), tron));
        assert!(!wish_matches(&w(Network::Tron, "H3PX", ""), tron));
        let eth = "0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf";
        assert!(wish_matches(&w(Network::Eth, "7E5F", ""), eth));
        assert!(wish_matches(&w(Network::Eth, "0x7E5F", "Bdf"), eth));
        assert!(!wish_matches(&w(Network::Eth, "7e5f", ""), eth));
        let bc1 = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";
        assert!(wish_matches(&w(Network::BtcP2wpkh, "w508", ""), bc1));
        assert!(wish_matches(&w(Network::BtcP2wpkh, "bc1qw508", ""), bc1));
        assert!(charset_ok(Network::Tron, "Pet") && !charset_ok(Network::Tron, "P0t"));
        assert!(charset_ok(Network::Eth, "cafe") && !charset_ok(Network::Eth, "cafg"));
        assert!(!charset_ok(Network::BtcP2tr, "b"));
    }

    /// Подсветка заказанных частей адреса: начало вместе с постоянной частью
    /// сети и конец; чужой адрес не подсвечивается.
    #[test]
    fn wish_highlight() {
        let w = |start: &str, end: &str| Wish { net: Network::Tron, start: start.into(), end: end.into() };
        let a = "TFFFF2PrLxAvQruE3uHzcjS1QVPnTxFXEF";
        assert_eq!(wish_marks(Some(&w("FFFF", "F")), a), (5, 1));
        assert_eq!(wish_marks(Some(&w("TFFFF", "")), a), (5, 0));
        assert_eq!(wish_marks(Some(&w("", "XEF")), a), (0, 3));
        assert_eq!(wish_marks(Some(&w("Pet", "")), a), (0, 0));
        assert_eq!(wish_marks(None, a), (0, 0));
        let job = address_job(a, 5, 1, 16.0);
        assert_eq!(job.text, a);
    }

    /// Постоянное начало каждой сети действительно есть у её адресов, а
    /// заказ на новые сети понимается так же, как у старых.
    #[test]
    fn fixed_start_matches_real_addresses() {
        use secp256k1::{PublicKey, Secp256k1, SecretKey};
        let secp = Secp256k1::new();
        for n in 1..40u8 {
            let pk = PublicKey::from_secret_key(&secp, &SecretKey::from_slice(&[n; 32]).unwrap());
            for net in Network::ALL {
                let a = split_key::address(net, &pk);
                assert!(a.starts_with(fixed_start(net)), "{net:?}: {a}");
                assert!(charset_ok(net, wanted_body(net, &a)), "{net:?}: {a}");
                // Предпросмотр знает обычную длину адреса (Base58 бывает на 1–2 короче).
                assert!(a.len() <= typical_len(net) && a.len() + 2 >= typical_len(net), "{net:?}: {a}");
                let w = |start: &str, end: &str| Wish { net, start: start.into(), end: end.into() };
                assert!(wish_matches(&w(&a[..fixed_start(net).len() + 2], &a[a.len() - 2..]), &a));
            }
        }
        let w = |net, start: &str| Wish { net, start: start.into(), end: String::new() };
        let bsc = "0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf";
        assert!(wish_matches(&w(Network::Bsc, "0X7E5F"), bsc) && !wish_matches(&w(Network::Bsc, "7e5f"), bsc));
        assert!(charset_ok(Network::Cosmos, "qqq") && !charset_ok(Network::Cosmos, "b"));
        assert!(charset_ok(Network::Xrp, "Pet") && !charset_ok(Network::Xrp, "P0t"));
    }
}
