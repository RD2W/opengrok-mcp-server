# Использование

Назад: [← Установка](./installation.md)
Далее: [Архитектура →](./architecture.md)

---

## Версия

```bash
opengrok-mcp --version
```

Вывод включает версию, автора, хэш коммита, дату сборки и целевую платформу.

> **Примечание для Docker:** при ручной сборке без `--build-arg GIT_HASH`
> хэш коммита будет показан как `unknown`. Для корректного отображения
> передавайте хэш при сборке:
>
> ```bash
> docker build --build-arg GIT_HASH=$(git rev-parse HEAD) .
> ```

## Справочник по конфигурации

Файл конфигурации: `config/config.toml`. Аннотированный шаблон: `config/config.example.toml`.
Переменные окружения переопределяют отдельные поля (перечислены ниже).

### `[opengrok]` — подключение

| Поле | Env var | По умолчанию | Описание |
|---|---|---|---|---|
| `base_url` | `OPENGROK_URL` | `""` | **Обязательно.** Базовый URL OpenGrok (например, `https://opengrok.example.com`) |
| `timeout_secs` | `OPENGROK_TIMEOUT_SECS` | `30` | Таймаут HTTP-запроса в секундах (макс. тело ответа: 50 МБ) |
| `ca_cert` | `OPENGROK_CA_CERT`, `SSL_CERT_FILE` | — | Путь к PEM-файлу пользовательского CA-сертификата (загружается дополнительно к системному trust store; приоритет `OPENGROK_CA_CERT`) |
| `ca_cert_dir` | `OPENGROK_CA_CERT_DIR`, `SSL_CERT_DIR` | — | Директория с файлами CA-сертификатов (.crt, .pem); приоритет `OPENGROK_CA_CERT_DIR` |
| `verify_ssl` | `OPENGROK_VERIFY_SSL` | `true` | Включение/отключение проверки TLS-сертификата (используйте `false`/`0`/`no` для отключения) |

### `[opengrok.auth]` — аутентификация

| Поле | По умолчанию | Описание |
|---|---|---|
| `mode` | `"none"` | `"token"`, `"basic"` или `"none"` |
| `token_env` | `"OPENGROK_TOKEN"` | Имя переменной окружения для Bearer-токена (только для `mode = "token"`) |
| `username_env` | — | Имя переменной окружения для логина Basic Auth (только для `mode = "basic"`; **должно быть задано**) |
| `password_env` | — | Имя переменной окружения для пароля Basic Auth (только для `mode = "basic"`; пустой если не задан) |

Учётные данные никогда не хранятся в файле конфигурации — только имена переменных окружения.

### `[service]` — поведение

| Поле | Env var | По умолчанию | Описание |
|---|---|---|---|
| `strip_html` | `MCP_STRIP_HTML` | `true` | Удаление HTML-тегов `<b>` из строк поиска (`true`/`false`/`1`/`0`/`yes`/`no`) |
| `max_hits_per_file` | `MCP_MAX_HITS_PER_FILE` | `10` | Максимум строк совпадений в одном файле |
| `default_max_results` | `MCP_DEFAULT_MAX_RESULTS` | `25` | Лимит результатов по умолчанию, если клиент не указал |

### `[cache]` — кэш в памяти (только результаты поиска)

| Поле | Env var | По умолчанию | Описание |
|---|---|---|---|
| `enabled` | `MCP_CACHE_ENABLED` | `false` | Включение/отключение кэша (`Mutex<LruCache>` с LRU-вытеснением) |
| `ttl_secs` | `MCP_CACHE_TTL_SECS` | `300` | Время жизни записи в секундах (ленивое удаление при доступе) |
| `max_entries` | `MCP_CACHE_MAX_ENTRIES` | `1000` | Максимум записей в кэше (LRU-вытеснение при переполнении) |

### `[rate_limit]` — ограничение частоты (token bucket, GCRA через governor)

| Поле | Env var | По умолчанию | Описание |
|---|---|---|---|
| `enabled` | `MCP_RATE_LIMIT_ENABLED` | `false` | Включение/отключение ограничения |
| `requests_per_second` | `MCP_RATE_LIMIT_RPS` | `5` | Устойчивая частота запросов |
| `burst` | `MCP_RATE_LIMIT_BURST` | `10` | Ёмкость всплеска |

### `[transport]` — режим сервера

| Поле | Env var | По умолчанию | Описание |
|---|---|---|---|
| `mode` | `MCP_TRANSPORT` | `"both"` | `"stdio"`, `"http"` или `"both"` |
| `bind_addr` | `MCP_BIND_ADDR` | `"127.0.0.1:8080"` | Адрес для HTTP |
| `http_path` | `MCP_HTTP_PATH` | `"/mcp"` | Путь эндпоинта MCP |
| `health_path` | `MCP_HEALTH_PATH` | `"/healthz"` | Эндпоинт живучести |
| `ready_path` | `MCP_READY_PATH` | `"/readyz"` | Эндпоинт готовности |
| `metrics_path` | `MCP_METRICS_PATH` | `"/metrics"` | Эндпоинт метрик Prometheus |
| `allowed_hosts` | `MCP_ALLOWED_HOSTS` | `[]` | Разрешённые значения заголовка Host (через запятую в env; защита от DNS rebinding) |
| `mcp_auth_token` | `MCP_AUTH_TOKEN` | `""` | Bearer-токен для аутентификации MCP-эндпоинта (пустая строка = отключено). Можно задать в TOML или через env. |

#### Аутентификация MCP-сервера (входящие запросы)

Задайте `mcp_auth_token` в `config.toml` или через переменную окружения `MCP_AUTH_TOKEN`.
Если значение не пустое, каждый входящий запрос к MCP-эндпоинту должен содержать заголовок:

```
Authorization: Bearer <токен>
```

Запросы без токена получают **HTTP 401 Unauthorized**. Сравнение токена —
`subtle::ConstantTimeEq` (timing-safe). Влияет только на HTTP-транспорт,
stdio-транспорт не затрагивается. Пустая строка отключает аутентификацию.

```toml
[transport]
mcp_auth_token = "shared-secret-12345"
```

Или через env: `export MCP_AUTH_TOKEN=shared-secret-12345`

> Это **входящая** аутентификация самого MCP-сервера — в отличие от секции
> `[opengrok.auth]`, которая настраивает **исходящую** аутентификацию к OpenGrok.

### `[log]`

| Поле | Env var | По умолчанию | Описание |
|---|---|---|---|
| `level` | `MCP_LOG_LEVEL`, `RUST_LOG` | `"info"` | `trace`, `debug`, `info`, `warn`, `error` (`RUST_LOG` применяется последним) |

---

## Режимы транспорта

### Режим stdio

```toml
[transport]
mode = "stdio"
```

Сервер читает MCP-сообщения из stdin и пишет в stdout. Используйте:

- С `docker exec` для контейнерных OpenGrok sidecar-ов
- С Claude Desktop или другими локальными MCP-клиентами
- Для отладки — легко передать тестовые JSON-сообщения через pipe

### Режим HTTP (Streamable HTTP)

```toml
[transport]
mode = "http"
bind_addr = "0.0.0.0:8080"
```

Сервер запускает HTTP-сервер с:

- MCP-эндпоинтом по настроенному `http_path` (`/mcp`)
- Проверкой живучести на `/healthz`
- Проверкой готовности на `/readyz`
- Метриками Prometheus на `/metrics`

Используйте для многоклиентского развёртывания, удалённого доступа или когда
MCP-клиент не поддерживает запуск процессов.

### Режим both

```toml
[transport]
mode = "both"
```

Запускает stdio и HTTP одновременно. Полезно для отладки HTTP-развёртываний:
канал stdio позволяет инспектировать трафик, пока HTTP-сервер обрабатывает
боевую нагрузку.

---

## Защита от DNS rebinding

В режиме HTTP rmcp проверяет заголовок `Host` по списку `allowed_hosts`.
Настройте его под своё развёртывание:

```toml
# Docker — клиенты подключаются по имени контейнера
allowed_hosts = ["localhost", "127.0.0.1", "opengrok-mcp", "opengrok-mcp:8004"]

# Публичное развёртывание за обратным прокси
allowed_hosts = ["localhost", "mcp.example.com"]
```

Пустой `allowed_hosts` принимает запросы от любого хоста.

---

## Health-эндпоинты

| Эндпоинт | Поведение |
|---|---|
| `GET /healthz` | Всегда `200 OK`, если процесс жив |
| `GET /readyz` | `200`, когда конфигурация загружена и OpenGrok отвечает на лёгкий пробный запрос; иначе `503` |
| `GET /metrics` | Текстовый формат Prometheus — счётчики запросов, задержки, попадания/промахи кэша |

### Проверка здоровья в Docker

```yaml
healthcheck:
  test: ["CMD", "wget", "-qO-", "http://localhost:8080/healthz"]
  interval: 30s
  retries: 3
```

---

## Справочник инструментов MCP

Сервер предоставляет **25 инструментов**, покрывающих всё OpenGrok REST API.

### Инструменты поиска

| Инструмент | Описание | Основные параметры |
|---|---|---|
| `search_code` | Полнотекстовый поиск (Lucene-синтаксис) | `query`, `project?`, `max_results` |
| `search_definition` | Поиск определения символа | `symbol`, `project?`, `max_results` |
| `search_references` | Поиск всех использований символа | `symbol`, `project?`, `max_results` |
| `search_file_path` | Поиск файлов по пути (glob) | `path`, `project?`, `max_results` |
| `search_history` | Поиск по истории изменений | `hist`, `project?`, `max_results` |
| `advanced_search` | Расширенный поиск (все поля, пагинация, сортировка) | `full?`, `def?`, `symbol?`, `path?`, `hist?`, `file_type?`, `project?`, `max_results?`, `start?`, `max_hits_per_file?`, `sort?` |
| `suggest` | Автодополнение поискового запроса | `project`, `field`, `caret`, `full?`, `defs?`, `refs?`, `path?`, `file_type?` |

### Инструменты для файлов

| Инструмент | Описание | Основные параметры |
|---|---|---|
| `get_file_content` | Получение содержимого файла | `project`, `path` |
| `get_file_definitions` | Определения (функции, классы) в файле | `path` |
| `get_file_genre` | Тип файла (PLAIN, XREFABLE, IMAGE, DATA, HTML) | `path` |
| `get_history` | История изменений файла | `path`, `start?`, `max?`, `with_files?` |
| `get_annotation` | Аннотация (blame) для файла | `path` |

### Инструменты для директорий и проектов

| Инструмент | Описание | Основные параметры |
|---|---|---|
| `list_directory` | Список содержимого директории | `path` |
| `list_indexed_projects` | Список индексированных проектов | — |
| `list_all_projects` | Список всех проектов (включая неиндексированные) | — |
| `list_groups` | Список групп проектов | — |
| `get_group_projects` | Проекты внутри группы (включая подгруппы) | `group` |
| `list_project_files` | Список файлов проекта из индекса | `project`, `path?` (по умолчанию: `"/"`) |
| `list_project_repos` | Пути репозиториев проекта | `project` |
| `get_project_property` | Per-project свойство | `project`, `name` |
| `get_repo_property` | Свойство репозитория (тип, ветка, remote, ...) | `field`, `repository` |

### Системные инструменты

| Инструмент | Описание | Основные параметры |
|---|---|---|
| `get_suggest_config` | Конфигурация suggester'а | — |
| `get_index_time` | Время последней индексации (ISO 8601) | — |
| `get_opengrok_version` | Версия OpenGrok | — |
| `health_check` | Проверка живости OpenGrok | — |

### Формат результатов

Все результаты поиска возвращаются в виде форматированного текста:
- Общее количество совпадений и результаты по файлам
- Номера строк и содержимое совпадающих строк
- Время поиска в миллисекундах
- Подсказки пагинации при наличии `has_more` в ответе API

HTML-теги удаляются из строк результатов, если `strip_html = true`.

### Пагинация

Используйте параметр `start` в `advanced_search` для offset-based пагинации:

```
Tool: advanced_search
full: "init_boot_images"
project: "aosp"
start: 25
```

Сервер прозрачно обрабатывает механику пагинации OpenGrok.
