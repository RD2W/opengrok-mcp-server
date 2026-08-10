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
│   │           ├── cache.rs   # TTL-кэш в памяти (Mutex<LruCache>)
│   │           ├── rate_limit.rs # Ограничитель частоты token bucket (governor)
│   │           └── format.rs  # Очистка HTML-тегов, форматирование результатов
│   └── opengrok-mcp/          # Бинарный крейт — слой MCP-сервера
│       └── src/
│           ├── main.rs        # Точка входа, аргументы CLI, инициализация логирования
│           ├── config.rs      # Загрузка TOML-конфигурации + переопределение через env
│           ├── mcp/
│           │   ├── mod.rs     # Инициализация MCP-сервера, диспетчеризация инструментов
│           │   ├── tools.rs   # Типы параметров инструментов (JSON Schema через schemars)
│           │   └── tools_impl/ # Реализации обработчиков по категориям
│           ├── transport/
│           │   ├── mod.rs     # Диспетчеризация транспорта (stdio / http / both)
│           │   ├── stdio.rs   # Транспорт stdin/stdout
│           │   └── http.rs    # Axum + rmcp Streamable HTTP транспорт
│           └── health.rs      # Эндпоинты /healthz, /readyz, /metrics
├── config/
│   ├── config.example.toml    # Аннотированный шаблон конфигурации
│   ├── .env.example           # Справочник переменных окружения
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

| Модуль | Назначение |
|---|---|
| `domain.rs` | Все типы данных: `SearchResult`, `FileContent`, `HistoryEntry`, `Project`, `DirectoryEntry`, типы ошибок (`DomainError`) |
| `application.rs` | Высокоуровневые операции: `search()`, `get_file_content()`, `get_history()`, с пагинацией, кэшированием и форматированием |
| `infrastructure/client.rs` | HTTP-клиент на `reqwest`: формирование запросов, добавление заголовков аутентификации, разбор ответов, обработка особенностей OpenGrok |
| `infrastructure/tls.rs` | TLS-конфигурация: загрузка пользовательских CA, настройка rustls, разбор PEM |
| `infrastructure/format.rs` | Очистка HTML-тегов (`<b>`, `<i>` и др.), форматирование всех типов ответов |
| `infrastructure/cache.rs` | Кэш в памяти с TTL-вытеснением на основе `Mutex<LruCache>` |
| `infrastructure/rate_limit.rs` | Ограничитель частоты token bucket через `governor` |

### `opengrok-mcp` — MCP-сервер

| Модуль | Назначение |
|---|---|
| `mcp/mod.rs` | Инициализация MCP-сервера, диспетчеризация обработчиков 25 инструментов |
| `mcp/tools.rs` | Типы параметров 25 инструментов с JSON Schema (schemars): имена, описания, значения по умолчанию |
| `mcp/tools_impl/` | Реализации обработчиков, сгруппированные по категориям (search, content, history, metadata, system) |
| `config.rs` | Загрузка конфигурации: разбор TOML, переопределение через env, валидация |
| `transport/http.rs` | Маршрутизатор Axum с `NeverSessionManager` (stateless, протокол MCP 2026-07-28): MCP-эндпоинт, health, readiness, metrics, middleware аутентификации по MCP-токену |
| `transport/stdio.rs` | Транспорт stdin/stdout через rmcp |
| `health.rs` | Обработчики health check: живучесть, готовность с пробным запросом к OpenGrok, сбор метрик Prometheus |
| `main.rs` | Точка входа: разбор CLI, инициализация конфигурации, выбор транспорта, обработка сигналов завершения |

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
  └── format.rs                  ← очистка HTML, форматирование результата
  │
  ▼
application.rs                   ← пагинация, формирование ответа
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

### Почему Mutex<LruCache> для кэша?

Путь кэширования задействуется только при поисковых операциях — обычно это вызовы
умеренной частоты. `Mutex<LruCache>` обеспечивает безопасный конкурентный доступ
с LRU-вытеснением — проще и достаточно для данного паттерна доступа по сравнению
с lock-free конкурентной картой.

### Почему governor для ограничения частоты?

`governor` реализует Generic Cell Rate Algorithm (GCRA) — вариант token bucket.
Он лёгкий, совместим с async и хорошо подходит для защиты одного бэкенда (OpenGrok)
от чрезмерной частоты запросов.
