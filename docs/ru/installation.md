# Установка

Далее: [Использование →](./usage.md)
Назад: [← Обзор](./overview.md)

---

## Требования

- **Rust** 1.97 или новее (edition 2024)
- Экземпляр **OpenGrok**, доступный по HTTP(S)
- **Docker** (опционально — для контейнерного развёртывания)

---

## Сборка из исходников

```bash
git clone <repo-url> opengrok-mcp-server
cd opengrok-mcp-server

# Сборка в release-режиме
cargo build --release

# Бинарный файл находится в:
#   target/release/opengrok-mcp
```

### Конфигурация

```bash
cp config/config.example.toml config/config.toml
```

Отредактируйте `config/config.toml` — как минимум, укажите:

```toml
[opengrok]
base_url = "https://your-opengrok.example.com"

[opengrok.auth]
mode = "token"   # или "basic" / "none"
token_env = "OPENGROK_TOKEN"
```

Учётные данные задаются через переменные окружения:

```bash
export OPENGROK_TOKEN="ваш-токен"
# или для Basic auth:
export OPENGROK_USERNAME="пользователь"
export OPENGROK_PASSWORD="пароль"
```

### Запуск

```bash
cargo run --release
```

Сервер запускается в режиме stdio по умолчанию и готов к подключению MCP-клиентов.

---

## Docker

### Локальная разработка

```bash
# Задайте учётные данные в config/.env:
#   OPENGROK_TOKEN=ваш-токен
#   OPENGROK_URL=https://opengrok.example.com

docker compose up -d
```

### Docker Hub (готовый образ)

Готовые multi-arch образы (linux/amd64, linux/arm64) публикуются на
[Docker Hub](https://hub.docker.com/r/rd2w/opengrok-mcp/tags) при каждом
релизе с тегом.

```bash
# Загрузка последнего релиза
docker pull rd2w/opengrok-mcp:latest

# Или конкретной версии
docker pull rd2w/opengrok-mcp:v0.1.0

# Используйте docker-compose файл для готовых образов
docker compose -f docker-compose.hub.yml up -d
```

Файл `docker-compose.hub.yml` идентичен `docker-compose.yml`, за исключением
использования `image:` вместо `build:` — Rust toolchain и компиляция на целевом
хосте не требуются.

### Удалённый / изолированный деплой

Для хостов **без доступа в интернет** (характерно для корпоративных сред) образ
собирается на машине с сетью, затем переносится как самодостаточный архив.
Многоэтапная Docker-сборка вкомпилировывает все зависимости в образ — сеть
во время работы не требуется.

```bash
# 1. Сборка на машине с доступом в интернет
#    (загрузка базовых образов, crates Rust, компиляция — всё вкомпилировано)
docker build -t opengrok-mcp:latest .

# 2. Экспорт в один переносимый архив (~35 МБ)
docker save opengrok-mcp:latest | gzip > opengrok-mcp.tar.gz

# 3. Перенос на изолированный хост (USB-накопитель, scp на jump host и т.д.)
scp opengrok-mcp.tar.gz docker-compose.yml remote-host:~/mcp/

# 4. На удалённом хосте — загрузка и запуск (интернет не нужен)
ssh remote-host
cd ~/mcp/
docker load < opengrok-mcp.tar.gz               # импорт образа

# Подготовка конфигурации
mkdir -p config
cp /path/to/config.toml config/                 # ваша конфигурация
cp /path/to/your-ca.crt config/certs/           # CA-сертификат (при использовании своего TLS)

# Создайте config/.env с учётными данными (никогда не коммитьте этот файл)
echo 'OPENGROK_TOKEN=ваш-токен' > config/.env
echo 'OPENGROK_URL=https://opengrok.example.com' >> config/.env

docker compose up -d
```

> **Чеклист для изолированной среды:** Образ включает базовый Alpine,
> `ca-certificates`, скомпилированный бинарник и все Rust-зависимости.
> Единственная внешняя зависимость — сам экземпляр OpenGrok: MCP-сервер
> выполняет **исходящие** HTTPS-запросы к нему, поэтому хост должен иметь
> сетевой доступ до OpenGrok (но не обязан иметь доступ в интернет в целом).

### Размер образа

Многоэтапная Docker-сборка создаёт образ на базе Alpine размером около
**35 МБ** — достаточно мал для удобной передачи по медленным каналам.

---

## TLS с пользовательскими сертификатами

Если ваш экземпляр OpenGrok использует корпоративный или самоподписанный сертификат:

1. Поместите PEM-файл CA-сертификата в `config/certs/`:
   ```bash
   cp your-ca.crt config/certs/
   ```

2. Укажите в `config.toml`:
   ```toml
   [opengrok]
   ca_cert = "./config/certs/your-ca.crt"
   ```

3. Или используйте переменные окружения:
   ```bash
   export OPENGROK_CA_CERT=/path/to/ca.pem
   export SSL_CERT_FILE=/path/to/ca.pem
   export SSL_CERT_DIR=/path/to/certs/
   ```

В Docker директория `config/` монтируется только для чтения — сертификаты
подхватываются автоматически.

### Отключение проверки TLS (небезопасно!)

Только для доверенных внутренних сетей:

```toml
[opengrok]
verify_ssl = false
```

Или `export OPENGROK_VERIFY_SSL=false`.
