# Разработка

Назад: [← Архитектура](./architecture.md)

---

## Начало работы

```bash
git clone <repo-url> opengrok-mcp-server
cd opengrok-mcp-server
cargo build --workspace
cargo test --workspace
```

Разработка ведётся в ветке `dev`. Создавайте feature-ветки от неё:

```bash
git checkout dev
git checkout -b feat/моя-фича
```

---

## Запуск тестов

```bash
# Все тесты (158 на момент написания)
cargo test --workspace

# Отдельный крейт
cargo test -p opengrok-core
cargo test -p opengrok-mcp

# С выводом
cargo test -- --nocapture

# Запуск игнорируемых (интеграционных) тестов
cargo test -- --ignored
```

---

## CI-пайплайн

CI запускается при каждом пуше в ветки `dev`, `main`, `ci` и для всех PR:

| Задача | Команда | Назначение |
|---|---|---|
| Форматирование | `cargo fmt --all -- --check` | Единый стиль кода |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | Поиск ошибок и стилистических проблем |
| Тесты | `cargo test --workspace --locked` | Запуск всех модульных и интеграционных тестов |
| Сборка | `cargo build --workspace --locked --release` | Проверка компиляции release-сборки |

Workflow GitHub Actions: `.github/workflows/ci.yml`

---

## Правила оформления кода

### Общие

- **Язык:** английские комментарии и сообщения коммитов
- **Коммиты:** [Conventional Commits](https://www.conventionalcommits.org/) —
  `feat:`, `fix:`, `chore:`, `docs:`, `test:`, `refactor:`
- **Форматирование:** `rustfmt` с настройками по умолчанию
- **Линтинг:** `clippy` с `-D warnings` — все предупреждения считаются ошибками в CI

### SPDX-заголовки

Каждый новый `.rs` файл должен начинаться с:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
```

Точный формат смотрите в любом существующем исходном файле.

### Организация модулей

- Небольшие, сфокусированные файлы с чёткими обязанностями
- Один основной тип или задача на файл
- `mod.rs` только для объявлений модулей и реэкспорта

---

## Добавление нового инструмента MCP

1. **Добавьте доменный тип** в `opengrok-core/src/domain.rs`, если API возвращает
   новую форму ответа.

2. **Добавьте метод клиента** в `opengrok-core/src/infrastructure/client.rs` —
   реализуйте HTTP-вызов к эндпоинту OpenGrok REST API.

3. **Добавьте метод приложения** в `opengrok-core/src/application.rs` —
   подключите кэширование, ограничение частоты и форматирование.

4. **Определите схему инструмента** в `opengrok-mcp/src/mcp/tools.rs`:
   ```rust
    #[tool(description = "Поиск определения символа по кодовой базе")]
    async fn search_definition(
       symbol: String,
       #[param(description = "Имя проекта для ограничения области поиска")]
       project: Option<String>,
   ) -> Result<CallToolResult, McpError> {
       // …
   }
   ```
   Используйте `#[param(description = "...")]` для каждого параметра — эти описания
   передаются LLM-клиентам и напрямую влияют на качество вызовов инструментов.

5. **Зарегистрируйте обработчик** в `opengrok-mcp/src/mcp/mod.rs` — добавьте
   инструмент в список инструментов сервера и свяжите с методом приложения.

6. **Добавьте тесты** — модульные тесты для доменного типа, интеграционные тесты
   для HTTP-клиента (с мок-ответом OpenGrok) и тесты обработчика для слоя MCP.

---

## Документация

При изменении поведения обновляйте:

- Соответствующие страницы в `docs/en/` и `docs/ru/` (поддерживайте синхронизацию)
- `README.md`, если изменения затрагивают быстрый старт или список возможностей
- `CHANGELOG.md` — добавляйте запись в раздел `[Unreleased]`
- `config/config.example.toml` — если меняются параметры конфигурации

---

## Чеклист перед PR

Перед открытием pull request выполните:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

Все четыре должны проходить. CI проверяет первые три — проверка сборки
отлавливает проблемы компиляции, которые тесты могут пропустить.

---

## Процесс релиза

Релизы автоматизированы через `.github/workflows/release.yml`:

1. Отправьте тег, например `v0.1.0`
2. CI собирает мультиархитектурные Docker-образы и создаёт GitHub Release
3. Бинарные артефакты прикрепляются к релизу

Ручные шаги (для отладки workflow):

```bash
docker build -t opengrok-mcp:v0.1.0 .
docker tag opengrok-mcp:v0.1.0 opengrok-mcp:latest
```
