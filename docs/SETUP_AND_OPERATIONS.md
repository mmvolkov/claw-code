# Настройка и эксплуатация Claw Code

Этот документ — канонический runbook по установке, запуску, конфигурации, аутентификации и режимам работы в текущем состоянии репозитория.

## Область охвата

Активная реализация находится в Rust workspace `rust/`.

Основные бинарники:

- `claw` — CLI и REPL runtime
- `claw-web` — локальный браузерный UI поверх того же Rust runtime

Основные рабочие поверхности:

- `.claw/sessions/` — сохраненные диалоги в рамках workspace
- `$HOME/.claw/credentials.json` — сохраненные OAuth credentials при локальном запуске
- смонтированный `/root/.claw/credentials.json` — сохраненные OAuth credentials в Docker
- `mock-anthropic-service` — детерминированный локальный сервис для parity и тестовых прогонов

## Матрица запуска

Используйте эту матрицу, чтобы выбрать правильный entrypoint.

| Цель | Бинарник | Рекомендуемый auth | Рекомендуемая команда |
| --- | --- | --- | --- |
| Интерактивный агент в терминале | `claw` | `ANTHROPIC_API_KEY` | `cargo run -p rusty-claude-cli --` |
| Интерактивный агент с локальной OpenAI-compatible LLM | `claw` | `OPENAI_API_KEY` | `cargo run -p rusty-claude-cli -- --provider openai-compatible --model local-model` |
| Одноразовый prompt для скрипта | `claw` | `ANTHROPIC_API_KEY` | `cargo run -p rusty-claude-cli -- prompt "summarize this repository"` |
| Browser UI | `claw-web` | `ANTHROPIC_API_KEY` | `cargo run -p web-api -- --cwd ..` |
| Только bootstrap OAuth credentials | `claw login` | browser OAuth | `cargo run -p rusty-claude-cli -- login` |
| Детерминированный локальный test harness | `mock-anthropic-service` | не требуется | `./scripts/run_mock_parity_harness.sh` |

Важное ограничение:

- Сохраненные Claude OAuth credentials сохраняются корректно.
- Для прямого inference против `https://api.anthropic.com/v1/messages` по-прежнему требуется `ANTHROPIC_API_KEY`.
- OAuth-only transport для inference в этом репозитории пока не реализован.
- Если вы направите `ANTHROPIC_BASE_URL` на совместимый прокси, принимающий bearer auth, то сохраненный OAuth или `ANTHROPIC_AUTH_TOKEN` могут использоваться и там.

## Локальная установка

### 1. Установите Rust toolchain

Для macOS:

```bash
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable
rustup component add clippy rustfmt
```

### 2. Соберите workspace

Из корня репозитория:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
cargo build --workspace
```

### 3. Проверьте доступные команды

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --help
./target/debug/claw-web --help
```

## Установка и запуск через Docker

Сжатая самодостаточная инструкция только для Docker: [DOCKER.md](./DOCKER.md).

В репозитории есть верхнеуровневый [Dockerfile](../Dockerfile). Образ включает:

- инструментарий Rust
- `cargo`
- `clippy`
- `rustfmt`
- `git`
- `python3`
- `ripgrep`
- установленный `claw`
- установленный `claw-web`

### Сборка образа

```bash
cd "$(git rev-parse --show-toplevel)"
docker build -t claw-code .
```

### Запуск CLI в Docker

```bash
cd "$(git rev-parse --show-toplevel)"
read -s ANTHROPIC_API_KEY
echo

docker run --rm -it \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  -v "$PWD":/workspace \
  claw-code
```

Для локального OpenAI-compatible backend:

```bash
cd "$(git rev-parse --show-toplevel)"
cp .env.example .env

docker run --rm -it \
  -v "$PWD":/workspace \
  claw-code \
  --provider openai-compatible
```

### Запуск web UI в Docker

Если контейнер **`claw-web`** уже существует, сначала выполните `docker rm -f claw-web`, иначе будет ошибка конфликта имени. В сценарии ниже перед `docker run` добавлена безопасная очистка (`2>/dev/null || true` скрывает сообщение, если контейнера ещё не было).

```bash
cd "$(git rev-parse --show-toplevel)"
read -s ANTHROPIC_API_KEY
echo
mkdir -p "$HOME/.claw-docker"
docker rm -f claw-web 2>/dev/null || true

docker run --rm -it \
  --name claw-web \
  -p 8787:8787 \
  -p 4545:4545 \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  -v "$PWD":/workspace \
  -v "$HOME/.claw-docker:/root/.claw" \
  --entrypoint claw-web \
  claw-code \
  --cwd /workspace
```

После запуска откройте `http://localhost:8787`.

### Инициализация OAuth в Docker

Если вы хотите сохранить Claude OAuth credentials в Docker-mounted auth directory:

```bash
cd "$(git rev-parse --show-toplevel)"
mkdir -p "$HOME/.claw-docker"

docker run --rm -it \
  -p 4545:4545 \
  -v "$PWD":/workspace \
  -v "$HOME/.claw-docker:/root/.claw" \
  claw-code login
```

Примечания:

- `-p 4545:4545` обязателен для callback `http://localhost:4545/callback`.
- Для стандартного direct API Anthropic этот OAuth-login сам по себе не делает runtime готовым к inference.
- Сохраненные credentials все равно полезны, если позже использовать bearer-capable upstream через `ANTHROPIC_BASE_URL`.

## Режимы аутентификации

### Режим 1. `ANTHROPIC_API_KEY`

Это стандартный рабочий режим для прямого доступа к Anthropic API.

Локальный запуск:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
read -s ANTHROPIC_API_KEY
echo
export ANTHROPIC_API_KEY
./target/debug/claw prompt "summarize this repository"
```

### Режим 2. `ANTHROPIC_AUTH_TOKEN`

Этот режим использует bearer token из переменных окружения.

Применяйте его только тогда, когда ваш upstream поддерживает bearer auth:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
read -s ANTHROPIC_AUTH_TOKEN
echo
export ANTHROPIC_AUTH_TOKEN
export ANTHROPIC_BASE_URL="http://127.0.0.1:8080"
./target/debug/claw prompt "status"
```

### Режим 3. Сохраненные OAuth credentials

`claw login` запускает браузерный OAuth flow и сохраняет credentials в:

- локально: `$HOME/.claw/credentials.json`
- в Docker: смонтированный `/root/.claw/credentials.json`

Команды:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw login
./target/debug/claw logout
```

Текущее поведение:

- OAuth credentials корректно сохраняются и загружаются.
- Web UI умеет отображать OAuth status.
- Для стандартного direct inference против Anthropic по-прежнему нужен `ANTHROPIC_API_KEY`.
- Если доступен только saved OAuth и base URL указывает на стандартный Anthropic API, UI помечает auth как `not inference-ready` и блокирует отправку запросов.

### Режим 4. OpenAI-compatible backend

Этот режим подходит для локальных LLM и любых upstream, совместимых с OpenAI `POST /v1/chat/completions`.

Нужны переменные окружения:

- `OPENAI_API_KEY`
- опционально `OPENAI_BASE_URL`
- опционально `OPENAI_MODEL`
- опционально `CLAW_SYSTEM_PROMPT`

CLI:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --provider openai-compatible prompt "summarize this repository"
```

Web UI:

- положите `OPENAI_API_KEY` и при необходимости `OPENAI_BASE_URL` в локальный `.env`
- в левом блоке `Controls` выберите `Provider = openai-compatible`
- в поле `Model` укажите model id вашего backend или задайте `OPENAI_MODEL`
- в поле `System Prompt` задайте инструкции для конкретного диалога или положите дефолт в `CLAW_SYSTEM_PROMPT`

Если одновременно присутствуют Anthropic credentials и локальный OpenAI-compatible backend, используйте явный `--provider openai-compatible` в CLI или selector `Provider` в web UI, чтобы не полагаться на `auto`.

### Режим 4. `GEMINI_API_KEY`

Этот режим использует Google Gemini через OpenAI-compatible transport.

Нужны переменные окружения:

- `GEMINI_API_KEY`
- опционально `GEMINI_BASE_URL`
- опционально `GEMINI_MODEL`

CLI:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --provider gemini prompt "summarize this repository"
```

### Режим 5. `DEEPSEEK_API_KEY`

Этот режим использует DeepSeek через OpenAI-compatible transport.

Нужны переменные окружения:

- `DEEPSEEK_API_KEY`
- опционально `DEEPSEEK_BASE_URL`
- опционально `DEEPSEEK_MODEL`

CLI:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --provider deepseek prompt "summarize this repository"
```

### Режим 6. `PERPLEXITY_API_KEY`

Этот режим использует Perplexity через OpenAI-compatible transport и endpoint `POST /chat/completions`.

Нужны переменные окружения:

- `PERPLEXITY_API_KEY`
- опционально `PERPLEXITY_BASE_URL`
- опционально `PERPLEXITY_MODEL`

CLI:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --provider perplexity prompt "Объясни разницу между REST и GraphQL в 5 пунктах."
```

### Приоритет источников credentials

Активный источник определяется в таком порядке:

1. `ANTHROPIC_API_KEY`
2. `ANTHROPIC_AUTH_TOKEN`
3. `OPENAI_API_KEY`
4. `XAI_API_KEY`
5. `GEMINI_API_KEY`
6. `DEEPSEEK_API_KEY`
7. `PERPLEXITY_API_KEY`
8. сохраненные OAuth credentials
9. отсутствие auth

Переменные окружения имеют приоритет над сохраненным OAuth и в CLI, и в web UI.

## Конфигурация

### Переменные окружения

Поддерживаемые операционные переменные:

- `ANTHROPIC_API_KEY` — прямой API key Anthropic
- `ANTHROPIC_AUTH_TOKEN` — bearer token для совместимого upstream
- `ANTHROPIC_BASE_URL` — кастомный API base URL или прокси
- `OPENAI_API_KEY` — credential для OpenAI-compatible backend
- `OPENAI_BASE_URL` — base URL OpenAI-compatible backend
- `OPENAI_MODEL` — provider-specific default model для OpenAI-compatible backend
- `XAI_API_KEY` — credential для xAI backend
- `XAI_BASE_URL` — base URL xAI backend
- `GEMINI_API_KEY` — credential для Gemini backend
- `GEMINI_BASE_URL` — base URL Gemini backend
- `GEMINI_MODEL` — default model для Gemini
- `DEEPSEEK_API_KEY` — credential для DeepSeek backend
- `DEEPSEEK_BASE_URL` — base URL DeepSeek backend
- `DEEPSEEK_MODEL` — default model для DeepSeek
- `PERPLEXITY_API_KEY` — credential для Perplexity backend
- `PERPLEXITY_BASE_URL` — base URL Perplexity backend
- `PERPLEXITY_MODEL` — default model для Perplexity
- `CLAW_SYSTEM_PROMPT` — общий системный промпт по умолчанию для CLI и Web UI

### Порядок загрузки конфигурации

Runtime загружает конфигурацию в таком порядке, причем более поздние файлы переопределяют более ранние:

1. `~/.claw.json`
2. `~/.config/claw/settings.json`
3. `"$(git rev-parse --show-toplevel)/.claw.json"`
4. `"$(git rev-parse --show-toplevel)/.claw/settings.json"`
5. `"$(git rev-parse --show-toplevel)/.claw/settings.local.json"`

Используйте `.claw/settings.local.json` только для локальных переопределений на конкретной машине.

## Режимы работы

### 1. Интерактивный REPL

Лучший вариант для длительной интерактивной работы с сохранением сессий.

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw
```

Можно также сразу переопределить модель:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --model claude-opus-4-6
./target/debug/claw --model claude-sonnet-4-6
./target/debug/claw --provider openai-compatible --model local-model
./target/debug/claw --provider gemini
./target/debug/claw --provider deepseek
./target/debug/claw --provider perplexity
```

### 2. Режим одноразового prompt

Лучший вариант для shell-автоматизации или одиночного ответа.

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw prompt "summarize this repository"
./target/debug/claw "explain rust/crates/runtime/src/lib.rs"
```

### 3. JSON-режим для автоматизации

Лучший вариант для скриптов, оберток и CI-адаптеров.

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --output-format json prompt "status"
```

### 4. Режим resume и обслуживания сессий

Подходит для инспекции или обслуживания сохраненной сессии без входа в REPL.

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --resume latest
./target/debug/claw --resume latest /status /diff
./target/debug/claw --resume latest /compact
```

### 5. Режим browser UI

Подходит, если нужен постоянный локальный браузерный интерфейс со streaming text и tool events.

Локальный запуск:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
cargo run -p web-api -- --cwd ..
```

Текущие возможности web UI:

- поле модели
- селектор режима прав
- поле для списка разрешенных инструментов
- включение/отключение tool use на текущий turn
- список сессий
- ручное обновление списка сессий
- создание новой сессии
- уплотнение текущей сессии
- панель OAuth status
- SSE-стриминг через `/api/chat/stream`

### 6. Режим mock parity harness

Подходит для детерминированной проверки без обращения к live Anthropic API.

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./scripts/run_mock_parity_harness.sh
```

Ручной запуск mock-сервиса:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
cargo run -p mock-anthropic-service -- --bind 127.0.0.1:0
```

## CLI против Web UI

Ниже практическое сравнение двух основных способов работы с runtime.

### Работа через CLI

CLI — это самый прямой интерфейс к runtime.

Типичный запуск с локальной OpenAI-compatible моделью:

```bash
cd "$(git rev-parse --show-toplevel)"

docker run --rm -it \
  -v "$PWD":/workspace \
  claw-code \
  --provider openai-compatible
```

Типичный рабочий цикл:

1. Запустить REPL.
2. Отправлять prompt напрямую из терминала.
3. Менять состояние через slash-команды вроде `/model`, `/permissions`, `/session list`.
4. Возвращаться к последней сессии через `claw --resume latest`.

Преимущества CLI:

- минимальная задержка и минимум лишнего интерфейса
- хорошо подходит для shell-скриптов и повторяемых команд
- есть JSON-режим для автоматизации
- проще использовать в SSH, tmux, CI и удаленных окружениях

### Работа через Web UI

Web UI использует тот же runtime, но добавляет визуальный слой над сессиями, auth-status и streaming events.

Типичный запуск:

```bash
cd "$(git rev-parse --show-toplevel)"
mkdir -p "$HOME/.claw-docker"

docker run --rm -it \
  -p 8787:8787 \
  -p 4545:4545 \
  -v "$PWD":/workspace \
  -v "$HOME/.claw-docker:/root/.claw" \
  --entrypoint claw-web \
  claw-code \
  --cwd /workspace
```

Типичный рабочий цикл:

1. Открыть `http://localhost:8787`.
2. Выбрать `Provider`, `Model`, `Permission Mode`.
3. При необходимости ограничить инструменты через `Allowed Tools`.
4. Отправить prompt и наблюдать streaming text, tool use, tool result и usage.
5. Переключаться между сессиями через список слева.

Преимущества Web UI:

- наглядный streaming-ответ в браузере
- удобно видеть tool activity без чтения терминального потока
- проще переключать provider и model интерактивно
- удобнее вручную просматривать и продолжать сохраненные сессии

### Что выбрать

- Выбирайте CLI, если важны скорость, repeatability и интеграция со скриптами.
- Выбирайте CLI, если работаете удаленно или в полностью терминальном окружении.
- Выбирайте Web UI, если важны визуальный контроль, streaming events и быстрое интерактивное переключение настроек.
- Выбирайте Web UI, если показываете систему другим людям или исследуете поведение инструментов вживую.

## Режимы прав

Runtime поддерживает три режима прав:

- `read-only` — только чтение и поиск
- `workspace-write` — можно изменять файлы внутри workspace
- `danger-full-access` — неограниченный локальный доступ

Примеры CLI:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --permission-mode read-only prompt "summarize Cargo.toml"
./target/debug/claw --permission-mode workspace-write prompt "update the README"
./target/debug/claw --permission-mode danger-full-access prompt "inspect and fix the failing tests"
```

В web UI:

- селектор использует те же три значения
- запрос отвергается, если передан неподдерживаемый режим прав

## Режимы работы инструментов

### Все стандартные инструменты включены

Такое поведение используется по умолчанию, если вы не вводите ограничений.

### Ограниченный набор инструментов

Используйте `--allowedTools` в CLI или поле `Allowed Tools` в web UI.

CLI:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --allowedTools read,glob,grep "inspect the runtime crate"
```

Web UI:

- вводите список через запятую, например `read,glob,grep`
- очистите поле, чтобы убрать ограничение

### Отключение tool use для turn

Только в web UI:

- снимите галочку `Enable tool use for this chat turn`
- turn будет выполнен как чистый запрос к модели без запуска инструментов

## Модели

Поддерживаемые короткие alias в CLI:

- `opus` -> `claude-opus-4-6`
- `sonnet` -> `claude-sonnet-4-6`
- `haiku` -> `claude-haiku-4-5-20251213`

Эквивалентные примеры CLI:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --model opus prompt "review this diff"
./target/debug/claw --model claude-opus-4-6 prompt "review this diff"
```

В web UI поле модели ожидает полный идентификатор модели.

## Сессии и хранилище

### Хранилище сессий

Сохраненные диалоги лежат внутри текущего workspace:

- `"$(git rev-parse --show-toplevel)/.claw/sessions/"`

REPL автоматически сохраняет туда turn’ы.

### Хранилище credentials

При локальном запуске:

- `$HOME/.claw/credentials.json`

При запуске в Docker с документированным volume mount:

- `$HOME/.claw-docker/credentials.json` на хосте
- `/root/.claw/credentials.json` внутри контейнера

### Команды для работы с сессиями

Полезные slash-команды:

- `/help`
- `/status`
- `/sandbox`
- `/model`
- `/permissions`
- `/cost`
- `/resume`
- `/session`
- `/config`
- `/memory`
- `/diff`
- `/export`
- `/agents`
- `/mcp`
- `/skills`

## Элементы управления web UI

Текущий [index.html](../rust/crates/web-api/static/index.html) содержит следующие элементы:

- `Login With Claude` — запускает браузерный OAuth flow через loopback callback
- `Clear Saved OAuth` — удаляет сохраненные OAuth credentials
- `Model` — полный ID модели
- `Provider` — `auto`, `anthropic`, `openai-compatible`, `xai`
- `Permission Mode` — `danger-full-access`, `workspace-write`, `read-only`
- `Allowed Tools` — allow-list runtime-инструментов через запятую
- `Enable tool use for this chat turn` — отключает выполнение инструментов на текущем turn
- `Refresh` — перечитывает сохраненные сессии
- `New` — создает новую локальную сессию
- `Compact Current Session` — выполняет compaction загруженной сессии
- `Send Prompt` — запускает turn со streaming по SSE

Status-области web UI показывают:

- активный источник auth
- готовность auth к inference
- корень workspace
- версию runtime
- контекст текущей даты runtime
- текущий session ID
- usage по сообщениям, если он есть

## Устранение неполадок

### `invalid x-api-key`

Причина:

- `ANTHROPIC_API_KEY` отсутствует, пустой или невалидный.

Исправление:

```bash
cd "$(git rev-parse --show-toplevel)"
read -s ANTHROPIC_API_KEY
echo
export ANTHROPIC_API_KEY
```

### `OAuth authentication is currently not supported`

Причина:

- доступен только сохраненный OAuth
- runtime все еще ходит напрямую в стандартный Anthropic Messages API

Исправление:

- используйте `ANTHROPIC_API_KEY` для прямого доступа к Anthropic
- или задайте `ANTHROPIC_BASE_URL` на совместимый upstream, поддерживающий bearer auth

### Браузерный callback сбрасывается на `http://localhost:4545/callback`

Причина:

- callback-порт не опубликован из Docker
- или все еще работает старый контейнер / старый образ

Исправление:

```bash
cd "$(git rev-parse --show-toplevel)"
docker rm -f claw-web 2>/dev/null || true
docker build -t claw-code .
```

Затем перезапустите контейнер:

```bash
cd "$(git rev-parse --show-toplevel)"
docker run --rm -it \
  -p 4545:4545 \
  -p 8787:8787 \
  -v "$PWD":/workspace \
  -v "$HOME/.claw-docker:/root/.claw" \
  --entrypoint claw-web \
  claw-code \
  --cwd /workspace
```

### Web UI показывает, что auth сохранен, но `Send Prompt` недоступен

Причина:

- сохраненный OAuth существует
- direct inference через стандартный Anthropic не поддерживается в OAuth-only режиме в текущем состоянии репозитория

Исправление:

- запускайте с `ANTHROPIC_API_KEY`
- или настройте bearer-capable upstream через `ANTHROPIC_BASE_URL`

### Застоявшееся состояние сессий или auth в Docker

Очистка host-mounted auth state:

```bash
rm -f "$HOME/.claw-docker/credentials.json"
```

Очистка локальных workspace-сессий:

```bash
rm -rf "$(git rev-parse --show-toplevel)/.claw/sessions"
```

## Проверка

Запускайте форматирование, линтер и тесты из Rust workspace:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Если полный `cargo test --workspace` слишком тяжел для текущего лимита памяти Docker, то как минимум прогоните:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
cargo test -p api -p web-api
```

## Канонические entrypoint-документы

Используйте следующие файлы как актуальные точки входа в документацию:

- [README.md](../README.md) — обзор репозитория
- [USAGE.md](../USAGE.md) — краткое практическое руководство
- [SETUP_AND_OPERATIONS.md](./SETUP_AND_OPERATIONS.md) — подробный runbook
- [rust/README.md](../rust/README.md) — обзор Rust workspace
