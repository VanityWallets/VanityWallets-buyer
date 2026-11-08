# Vanity Buyer — split-key vanity wallet tool

**[English](#english)** · **[Русский](#русский)**

---

# English

## How it works — in short

- A wallet's private key is a big secret number. Whoever has that number owns the wallet.
- Here the key is made of two halves: **your half + the seller's half = the wallet key**.
- The program creates your half on your computer and keeps it in the request file. The seller only gets
  its "fingerprint" (the public half). It is enough to search for a nice address, but the half itself
  cannot be recovered from it — the security of every Bitcoin, Ethereum and TRON wallet rests on this.
- The seller picks their half so that the address looks nice and sends it to you after payment.
  The halves are added together on your computer, so the seller never has the full key and never can.

## Networks

- TRON; Ethereum and EVM networks: BNB Smart Chain, Polygon, Arbitrum, Optimism, Base, Avalanche C-Chain,
  Linea, zkSync Era, HyperEVM (the address and key are the same in all EVM networks);
- Bitcoin (1…, 3…, bc1q…, bc1p…), Litecoin (L…, M…, ltc1q…), Dogecoin, Dash;
- XRP Ledger — the key is given as a regular secp256k1 private key in hex, not as an "s…" secret: import
  it into a wallet that accepts a hex XRP key and check the address before funding it;
- Cosmos networks: Cosmos Hub, Osmosis, Celestia, dYdX, Akash, Juno, Stargaze, Noble (import the key into Keplr).

## Security

- **The program works without the internet.** It contains no network code: you can disconnect from the
  internet before starting it — everything works the same. The internet is needed only once — while
  building, to download the libraries (see below).
- Your half of the key is created by the Windows random number generator and is stored only on your
  computer — in the `orders` folder next to the program (if that folder is not writable — in
  `%LOCALAPPDATA%\VanityBuyer\orders`).
- The program never opens a web browser and never starts other programs: the `webbrowser` library is
  replaced with an empty stub for that (the `stubs` folder).
- All the code is open and small. All the math is in a single file, `src/split_key.rs`.

## Building the program

When you build the program yourself, you know for sure that you run exactly this code.

### 1. What to install

You need a computer with **Windows 10 or 11, 64-bit**.

1. Open the official website **https://www.rust-lang.org/tools/install** and download `rustup-init.exe`.
2. Run it. If the installer offers to install **Visual Studio Build Tools** (the C++ linker for Windows),
   agree: Rust cannot build programs on Windows without them. In the Visual Studio installer it is enough
   to tick **"Desktop development with C++"**.
3. In the Rust installer choose the default option (**1) Proceed with standard installation**).
4. After the installation, open a command prompt window.

Check — in the command prompt (`cmd` or PowerShell) run: rustc --version

A version should appear, for example `rustc 1.89.0`.

### 2. Building

1. Unpack the archive into any folder.
2. Open a command prompt in this folder. The easiest way: open the folder in File Explorer, click the
   address bar, type `cmd` and press Enter.
3. Run: cargo build --release --locked

What happens:

- Cargo (the Rust build tool) downloads the libraries from the official `crates.io` registry. The
  `--locked` flag means "take exactly the versions listed in `Cargo.lock` and update nothing". Every
  library is verified against its fingerprint from the same file.
- The first build takes a few minutes. At the end you will see the line `Finished release ...`.

The ready program will appear here: ...\target\release\pokupatel.exe

You can copy it to any folder you like. After that the `target` folder can be deleted.

## How to make sure it is honest

- Read `src/split_key.rs` — all the math is there: creating the codes, adding the halves, deriving addresses.
- Make sure the code does not use the network: there is not a single network call in it.
- Disconnect from the internet and use the program offline (if you like).
- Build the program yourself from these sources instead of using a ready-made exe.

## Files

| File | What it is |
|---|---|
| `src/main.rs` | The program window: tabs, checks, hints |
| `src/split_key.rs` | The math: request, confirmation and key-half codes, address derivation |
| `src/vault.rs` | Creating and reading request files |
| `Cargo.toml` | The list of libraries |
| `Cargo.lock` | Exact versions and fingerprints of all libraries |
| `build.rs` | Embeds the icon and description into the exe |
| `assets/` | The program icon |
| `stubs/webbrowser/` | An empty stub instead of the browser-opening library: the program never opens links or starts other programs |

## Troubleshooting

- **`'cargo' is not recognized`** — Rust is not installed, or the command prompt was opened before the
  installation. Close it and open it again.
- **`link.exe not found`** or a linker error — Visual Studio Build Tools with the "Desktop development
  with C++" component are not installed.
- **Error while downloading libraries** — the first build needs the internet. The program itself works
  without it after the build.
- **`the lock file needs to be updated`** — the files were changed or damaged. Take the sources again
  from the original archive.

---

# Русский

## Как это устроено — коротко

- Приватный ключ кошелька — это большое секретное число. У кого оно, тот и владеет кошельком.
- Здесь ключ складывается из двух половин: **ваша половина + половина продавца = ключ кошелька**.
- Вашу половину программа создаёт на вашем компьютере и хранит в файле заявки. Продавцу уходит только
  её «отпечаток» (публичная половина). По нему можно искать красивый адрес, но саму половину из него
  получить невозможно — на этом держится безопасность всех кошельков Bitcoin, Ethereum и TRON.
- Продавец подбирает свою половину так, чтобы адрес получился красивым, и присылает её вам после оплаты.
  Сложение половин происходит у вас, поэтому полного ключа у продавца нет и быть не может.

## Сети

- TRON; Ethereum и EVM-сети: BNB Smart Chain, Polygon, Arbitrum, Optimism, Base, Avalanche C-Chain,
  Linea, zkSync Era, HyperEVM (адрес и ключ во всех EVM-сетях одинаковые);
- Bitcoin (1…, 3…, bc1q…, bc1p…), Litecoin (L…, M…, ltc1q…), Dogecoin, Dash;
- XRP Ledger — ключ выдаётся как обычный приватный ключ secp256k1 в hex, а не секрет «s…»: импортируйте
  его в кошелёк, который принимает hex-ключ XRP, и сверьте адрес перед пополнением;
- Cosmos-сети: Cosmos Hub, Osmosis, Celestia, dYdX, Akash, Juno, Stargaze, Noble (импорт ключа в Keplr).

## Безопасность

- **Программа работает без интернета.** В ней нет сетевого кода: можете отключить интернет перед
  запуском — всё будет работать так же. Интернет нужен только один раз — при сборке, чтобы скачать
  библиотеки (см. ниже).
- Ваша половина ключа создаётся генератором случайных чисел Windows и хранится только на вашем компьютере —
  в папке `orders` рядом с программой (если туда нельзя записывать — в `%LOCALAPPDATA%\VanityBuyer\orders`).
- Программа не открывает браузер и не запускает другие программы: для этого библиотека `webbrowser`
  заменена пустой заглушкой (папка `stubs`).
- Весь код открыт и небольшой. Вся математика — в одном файле `src/split_key.rs`.

## Сборка программы

Собирая программу сами, вы точно знаете, что запускаете ровно этот код.

### 1. Что нужно установить

Нужен компьютер с **Windows 10 или 11, 64 бит**.

1. Откройте официальный сайт **https://www.rust-lang.org/tools/install** и скачайте `rustup-init.exe`.
2. Запустите его. Если установщик предложит поставить **Visual Studio Build Tools** (компоновщик C++
   для Windows) — согласитесь: без них Rust на Windows не собирает программы. В установщике Visual
   Studio достаточно отметить **«Разработка классических приложений на C++»** (Desktop development
   with C++).
3. В установщике Rust выберите вариант по умолчанию (**1) Proceed with standard installation**).
4. После установки откройте окно командной строки.

Проверка — в командной строке (`cmd` или PowerShell) выполните команду: rustc --version

Должна появиться версия, например `rustc 1.89.0`.

### 2. Сборка

1. Распакуйте архив с файлами в любую папку.
2. Откройте командную строку в этой папке. Проще всего: откройте папку в Проводнике, щёлкните по
   адресной строке, напишите `cmd` и нажмите Enter.
3. Выполните: cargo build --release --locked

Что происходит:

- Cargo (сборщик Rust) скачивает библиотеки из официального хранилища `crates.io`. Ключ `--locked`
  означает: «бери ровно те версии, что указаны в файле `Cargo.lock`, и ничего не обновляй». Подлинность
  каждой библиотеки проверяется по отпечатку из того же файла.
- Первая сборка занимает несколько минут. В конце появится строка `Finished release ...`.

Готовая программа появится здесь: ...\target\release\pokupatel.exe

Её можно скопировать в любую удобную папку. Папку `target` после этого можно удалить.

## Как самому убедиться в честности

- Прочитайте `src/split_key.rs` — там вся математика: создание кодов, сложение половин, вывод адресов.
- Убедитесь, что в коде нет работы с сетью: в нём нет ни одного сетевого вызова.
- Отключите интернет и пользуйтесь программой без него (если хотите).
- Собирайте программу сами из этих исходников, а не берите готовый exe.

## Файлы

| Файл | Что это |
|---|---|
| `src/main.rs` | Окно программы: вкладки, проверки, подсказки |
| `src/split_key.rs` | Математика: коды заявки, подтверждения и половины ключа, вывод адресов |
| `src/vault.rs` | Создание и чтение файлов заявок |
| `Cargo.toml` | Список библиотек |
| `Cargo.lock` | Точные версии и отпечатки всех библиотек |
| `build.rs` | Встраивает иконку и описание в exe |
| `assets/` | Иконка программы |
| `stubs/webbrowser/` | Пустая заглушка вместо библиотеки открытия браузера: программа не открывает ссылки и не запускает другие программы |

## Если что-то не так

- **`'cargo' is not recognized`** — Rust не установлен или окно командной строки открыто до установки.
  Закройте его и откройте заново.
- **`link.exe not found`** или ошибка компоновщика — не установлены Visual Studio Build Tools с
  компонентом «Разработка классических приложений на C++».
- **Ошибка при скачивании библиотек** — для первой сборки нужен интернет. Сама программа после сборки
  работает без него.
- **`the lock file needs to be updated`** — файлы изменены или повреждены. Возьмите исходники заново
  из исходного архива.
