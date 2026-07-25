# Архитектура

Назад: [← Использование](./usage.md)
Далее: [Разработка →](./development.md)

---

## Структура workspace

```
opengrok-mcp-server/
├── crates/
│   ├── opengrok-core/         # Библиотека — без зависимостей MCP
│   │   └── src/
│   │       ├── domain.rs      # Модели данных: SearchResult, FileContent, Project, …
│   │       ├── application.rs # Сервисный слой: поиск, файловые операции, логика пагинации
│   │       └── infrastructure/
│   │           ├── client.rs  # HTTP-клиент для OpenGrok REST API
│   │           ├── tls.rs     # Построитель TLS-конфигурации (rustls + системные сертификаты)
│   │           ├── cache.rs   # TTL-кэш в памяти (DashMap)
│   │           ├── rate_limit.rs # Ограничитель частоты token bucket (governor)
│   │           └── format.rs  # Очистка HTML-тегов, нормализация результатов
│   └── opengrok-mcp/          # Бинарный крейт — слой MCP-сервера
│       └── src/
│           ├── main.rs        # Точка входа, аргументы CLI, инициализация логирования
│           ├── config.rs      # Загрузка TOML-конфигурации + переопределение через env
│           ├── mcp/
│           │   ├── mod.rs     # Инициализация MCP-сервера, диспетчеризация инструментов
│           │   └── tools.rs   # Определения инструментов (JSON Schema через schemars)
│           ├── transport/
│           │   ├── mod.rs     # Абстракция транспорта
│           │   ├── stdio.rs   # Транспорт stdin/stdout
│           │   └── http.rs    # Axum + rmcp Streamable HTTP транспорт
│           └── health.rs      # Эндпоинты /healthz, /readyz, /metrics
├── config/
│   ├── config.example.toml    # Аннотированный шаблон конфигурации
│   ├── config.toml            # Локальная конфигурация (в gitignore)
│   ├── .env                   # Секретные переменные окружения (в gitignore)
│   └── certs/                 # CA-сертификаты для TLS (в gitignore)
├── Dockerfile                 # Многоэтапная сборка на Alpine
└── docker-compose.yml         # Локальное dev-окружение
```

---

## Слоевая архитектура

```
┌─────────────────────────────────────┐
│         LLM-клиент (MCP)            │
├─────────────────────────────────────┤
│  opengrok-mcp (бинарный)            │
│  ├── transport/      stdio / HTTP   │
│  ├── mcp/tools.rs    схемы инструм. │
│  ├── mcp/mod.rs      обработчики    │
│  ├── config.rs       загрузка конф. │
│  └── health.rs       health/metrics │
├─────────────────────────────────────┤
│  opengrok-core (библиотека)         │
│  ├── application.rs  сервисный слой │
│  ├── domain.rs       модели данных  │
│  └── infrastructure/                │
│      ├── client.rs   HTTP-клиент    │
│      ├── tls.rs      настройка TLS  │
│      ├── cache.rs    кэш ответов    │
│      ├── rate_limit  ограничитель   │
│      └── format.rs   очистка HTML   │
├─────────────────────────────────────┤
│         OpenGrok API (REST)         │
└─────────────────────────────────────┘
```

### Направление зависимостей

`opengrok-mcp` зависит от `opengrok-core`. `opengrok-core` **не имеет зависимостей
MCP** — это чистая HTTP-клиентская библиотека, которую можно переиспользовать
в других контекстах.

---

## Зоны ответственности крейтов

### `opengrok-core` — домен и инфраструктура

| Модуль | Строк | Назначение |
|---|---|---|
| `domain.rs` | 1206 | Все типы данных: `SearchResult`, `FileContent`, `HistoryEntry`, `Project`, `DirectoryEntry`, типы ошибок (`CoreError`) |
| `application.rs` | 480 | Высокоуровневые операции: `search()`, `get_file_content()`, `get_history()`, с пагинацией, кэшированием и форматированием |
| `infrastructure/client.rs` | 930 | HTTP-клиент на `reqwest`: формирование запросов, добавление заголовков аутентификации, разбор ответов, обработка особенностей OpenGrok |
| `infrastructure/tls.rs` | 476 | TLS-конфигурация: загрузка пользовательских CA, настройка rustls, разбор PEM |
| `infrastructure/format.rs` | 479 | Очистка HTML-тегов (`<b>`, `<i>` и др.), нормализация текста результатов |
| `infrastructure/cache.rs` | 221 | Кэш в памяти с TTL-вытеснением на основе `DashMap` |
| `infrastructure/rate_limit.rs` | 110 | Ограничитель частоты token bucket через `governor` |

### `opengrok-mcp` — MCP-сервер

| Модуль | Строк | Назначение |
|---|---|---|
| `mcp/mod.rs` | 361 | Инициализация MCP-сервера, диспетчеризация обработчиков инструментов, маппинг ошибок (`CoreError` → коды ошибок MCP) |
| `mcp/tools.rs` | 194 | Определения типов инструментов с JSON Schema (schemars): имена, описания, типы параметров, значения по умолчанию |
| `config.rs` | 466 | Загрузка конфигурации: разбор TOML, переопределение через env, валидация |
| `transport/http.rs` | 67 | Маршрутизатор Axum: MCP-эндпоинт, health, readiness, metrics |
| `transport/stdio.rs` | 20 | Транспорт stdin/stdout через rmcp |
| `health.rs` | 165 | Обработчики health check: живучесть, готовность с пробным запросом к OpenGrok, сбор метрик Prometheus |
| `main.rs` | 124 | Точка входа: разбор CLI, инициализация конфигурации, выбор транспорта, обработка сигналов завершения |

---

## Поток данных

```
LLM-клиент
  │
  │  MCP-запрос: { tool: "search", params: { query: "...", project: "aosp" } }
  ▼
transport/stdio.rs или http.rs   ← получение MCP-сообщения
  │
  ▼
mcp/mod.rs                       ← маршрутизация по имени инструмента
  │
  ▼
opengrok-core::application.rs    ← сервисная логика, проверка кэша, ограничение частоты
  │
  ▼
opengrok-core::infrastructure/
  ├── cache.rs                   ← возврат из кэша при попадании
  ├── rate_limit.rs              ← ожидание при превышении лимита
  ├── client.rs                  ← HTTP-запрос к OpenGrok
  │     │
  │     ▼
  │   tls.rs                     ← TLS с пользовательским CA (если настроен)
  │     │
  │     ▼
  │   OpenGrok API
  │
  ▼
opengrok-core::infrastructure/
  └── format.rs                  ← очистка HTML, нормализация результата
  │
  ▼
application.rs                   ← пагинация, формирование ответа с has_more
  │
  ▼
mcp/mod.rs                       ← сериализация в MCP-ответ
  │
  ▼
transport/                       ← отправка ответа обратно LLM
  │
  ▼
LLM-клиент
```

---

## Проектные решения

### Почему два крейта?

Разделение на `opengrok-core` (библиотека) и `opengrok-mcp` (бинарный) исключает
зависимости MCP из ядра HTTP-клиента. Это означает:

- Ядро можно использовать в не-MCP контекстах (например, CLI-инструмент или веб-интерфейс)
- Ускоряется компиляция при работе с ядром
- Зависимости чётко разделены — `rmcp`, `axum`, `schemars` есть только в бинарном крейте

### Почему reqwest + rustls?

- `reqwest` — де-факто стандартный HTTP-клиент Rust: хорошо протестирован, асинхронный, с поддержкой TLS
- `rustls` — реализация TLS на чистом Rust: исключает проблемы линковки OpenSSL, особенно
  в Docker-сборках на Alpine
- `rustls-native-certs` обеспечивает интеграцию с системным хранилищем сертификатов при необходимости

### Почему DashMap для кэша?

`DashMap` — конкурентная хэш-таблица: позволяет чтение без блокировок и мелкогранулярные
блокировки для записи. Для MCP-сервера, обрабатывающего параллельные LLM-запросы, это
избегает конкуренции, которую создал бы `Mutex<RwLock<HashMap>>`.

### Почему governor для ограничения частоты?

`governor` реализует Generic Cell Rate Algorithm (GCRA) — вариант token bucket.
Он лёгкий, совместим с async и хорошо подходит для защиты одного бэкенда (OpenGrok)
от чрезмерной частоты запросов.
