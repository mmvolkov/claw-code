# Использование Claw Code

Это руководство описывает текущий Rust workspace в `rust/` и CLI-бинарник `claw`.

Полный runbook находится в [docs/SETUP_AND_OPERATIONS.md](./docs/SETUP_AND_OPERATIONS.md). Этот файл остается концентрированным quick-start руководством.

## Предварительные требования

- инструментарий Rust с `cargo`
- Один из вариантов:
  - `ANTHROPIC_API_KEY` для прямого доступа к API
  - `OPENAI_API_KEY` для OpenAI-compatible backend
  - `GEMINI_API_KEY` для Gemini через OpenAI-compatible transport
  - `DEEPSEEK_API_KEY` для DeepSeek через OpenAI-compatible transport
  - `PERPLEXITY_API_KEY` для Perplexity через OpenAI-compatible transport
  - `claw login` для OAuth-аутентификации
- Опционально: `ANTHROPIC_BASE_URL`, если вы работаете через прокси или локальный сервис
- Опционально: `OPENAI_BASE_URL`, если OpenAI-compatible сервер расположен не на стандартном `/v1`
- Опционально: `OPENAI_MODEL`, `GEMINI_MODEL`, `DEEPSEEK_MODEL`, `PERPLEXITY_MODEL`
- Опционально: `CLAW_SYSTEM_PROMPT`, если нужен общий системный промпт для CLI и Web UI

## Локальный `.env`

Скопируйте шаблон из корня репозитория и заполните только нужные переменные:

```bash
cd /Users/michaelvolkov/projects/claw-code
cp .env.example .env
```

`claw` ищет `.env` в текущей директории и поднимается вверх по дереву. Поэтому запуск из `/Users/michaelvolkov/projects/claw-code/rust` тоже подхватит `/Users/michaelvolkov/projects/claw-code/.env`. `claw-web` загружает `.env` из `--cwd`.

## Сборка workspace

```bash
cd rust
cargo build --workspace
```

После debug-сборки CLI-бинарник будет доступен по пути `rust/target/debug/claw`.

## Запуск через Docker

В репозитории есть верхнеуровневый `Dockerfile`, позволяющий запускать `claw` в контейнеризированной dev-среде.

Соберите образ из корня репозитория:

```bash
cd /Users/michaelvolkov/projects/claw-code
docker build -t claw-code .
```

Запустите CLI в интерактивном режиме, смонтировав текущий репозиторий в `/workspace`:

```bash
cd /Users/michaelvolkov/projects/claw-code
read -s ANTHROPIC_API_KEY
echo

docker run --rm -it \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  -v "$PWD":/workspace \
  claw-code
```

Одноразовый prompt:

```bash
cd /Users/michaelvolkov/projects/claw-code
read -s ANTHROPIC_API_KEY
echo

docker run --rm -it \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  -v "$PWD":/workspace \
  claw-code prompt "summarize this repository"
```

Внутри образа остаются Rust toolchain, `git`, `python3` и `rg`, поэтому контейнер подходит не только как обертка над бинарником, но и как практическое рабочее окружение для `claw`.

### Запуск web-интерфейса

Rust workspace также включает браузерный UI, который обслуживается новым бинарником `claw-web`.

Запуск из корня репозитория:

```bash
cd /Users/michaelvolkov/projects/claw-code
mkdir -p "$HOME/.claw-docker"

docker run --rm -it \
  -p 4545:4545 \
  -p 8787:8787 \
  -v "$PWD":/workspace \
  -v "$HOME/.claw-docker:/root/.claw" \
  --entrypoint claw-web \
  claw-code \
  --cwd /workspace
```

После этого откройте `http://localhost:8787` в браузере.

Web-интерфейс стримит дельты ответа ассистента, активность инструментов, обновления usage и финальное завершение turn через SSE endpoint `/api/chat/stream`.

Для локальной OpenAI-compatible модели выберите в web UI `Provider = openai-compatible`, укажите model id вашей модели и передайте `OPENAI_API_KEY` плюс при необходимости `OPENAI_BASE_URL`.

Если вы хотите использовать браузерный Claude OAuth из web UI, не передавайте в контейнер `ANTHROPIC_API_KEY` и `ANTHROPIC_AUTH_TOKEN`, потому что переменные окружения имеют приоритет над сохраненными OAuth-токенами.  
Claw Web завершает OAuth через тот же loopback callback, что и `claw login`: `http://localhost:4545/callback`.  
При запуске в Docker callback-порт должен публиковаться через `-p 4545:4545`, иначе браузер не сможет вернуть OAuth redirect внутрь контейнера.  
Сохраненные Claude OAuth credentials сохраняются корректно, но этот runtime по умолчанию все еще отправляет прямые запросы в Anthropic Messages API. Для прямого inference против `https://api.anthropic.com` вам все равно нужен `ANTHROPIC_API_KEY`; OAuth-only transport для inference в этом проекте пока не реализован.

Для локального запуска без Docker:

```bash
cd rust
cargo run -p web-api -- --cwd ..
```

## Быстрый старт

### Интерактивный REPL

```bash
cd rust
./target/debug/claw
```

### Одноразовый prompt

```bash
cd rust
./target/debug/claw prompt "summarize this repository"
```

### Сокращенный prompt-режим

```bash
cd rust
./target/debug/claw "explain rust/crates/runtime/src/lib.rs"
```

### Вывод JSON для скриптов

```bash
cd rust
./target/debug/claw --output-format json prompt "status"
```

### OpenAI-compatible backend

```bash
cd /Users/michaelvolkov/projects/claw-code/rust
./target/debug/claw --provider openai-compatible prompt "summarize this repository"
```

## CLI и Web UI

### Когда использовать CLI

CLI лучше подходит, если вы:

- работаете в терминале и хотите максимальную скорость
- запускаете one-shot prompt из shell-скриптов
- хотите легко ограничивать инструменты через флаги
- используете `--output-format json` для автоматизации

Пример интерактивного запуска через Docker с локальной OpenAI-compatible моделью:

```bash
cd /Users/michaelvolkov/projects/claw-code

docker run --rm -it \
  -v "$PWD":/workspace \
  claw-code \
  --provider openai-compatible
```

Пример работы:

- начать диалог и задавать промпты прямо в REPL
- использовать `/status`, `/model`, `/permissions`, `/session list`
- выходить и возвращаться через `claw --resume latest`

### Когда использовать Web UI

Web UI лучше подходит, если вы:

- хотите видеть поток ответа и tool events в браузере
- хотите быстро переключать provider, model и permission mode без перезапуска процесса
- работаете с несколькими сохраненными сессиями и хотите визуально их просматривать
- предпочитаете интерфейс с контролами вместо флагов командной строки

Пример запуска через Docker:

```bash
cd /Users/michaelvolkov/projects/claw-code
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

Пример работы:

- открыть `http://localhost:8787`
- выбрать `Provider = openai-compatible`
- убедиться, что `Model` автоматически подставился из локального `.env`
- при необходимости задать `System Prompt`
- при необходимости поменять `Permission Mode` и `Allowed Tools`
- отправить prompt и смотреть streaming-ответ, tool events и usage

### Сравнение

- CLI быстрее для опытной работы, shell-автоматизации и repeatable-команд.
- CLI удобнее, когда важны copy-paste команды, флаги и интеграция со скриптами.
- Web UI удобнее для живого наблюдения за turn, потому что там сразу видны streaming text, tool activity и текущая сессия.
- Web UI удобнее для интерактивного выбора provider и model, особенно если вы часто переключаетесь между Anthropic и локальной LLM.
- CLI проще для CI, cron и одноразовых задач.
- Web UI проще для ручной исследовательской работы и демонстраций.

## Управление моделью и правами

```bash
cd rust
./target/debug/claw --model sonnet prompt "review this diff"
./target/debug/claw --provider openai-compatible --model local-model prompt "review this diff"
./target/debug/claw --provider gemini prompt "summarize this repository"
./target/debug/claw --provider deepseek prompt "summarize this repository"
./target/debug/claw --provider perplexity prompt "summarize this repository"
./target/debug/claw --system-prompt-text "Be precise and concise." prompt "review this diff"
./target/debug/claw --permission-mode read-only prompt "summarize Cargo.toml"
./target/debug/claw --permission-mode workspace-write prompt "update README.md"
./target/debug/claw --allowedTools read,glob "inspect the runtime crate"
```

Поддерживаемые режимы прав:

- `read-only`
- `workspace-write`
- `danger-full-access`

Поддерживаемые alias моделей в CLI:

- `opus` → `claude-opus-4-6`
- `sonnet` → `claude-sonnet-4-6`
- `haiku` → `claude-haiku-4-5-20251213`

Поддерживаемые provider-значения:

- `auto`
- `anthropic`
- `openai-compatible`
- `xai`
- `gemini`
- `deepseek`
- `perplexity`

Если `--provider openai-compatible` указан без `--model`, CLI попробует взять модель из `OPENAI_MODEL`. Для `gemini`, `deepseek` и `perplexity` аналогично используются `GEMINI_MODEL`, `DEEPSEEK_MODEL` и `PERPLEXITY_MODEL`.

## Аутентификация

### Ключ API

```bash
read -s ANTHROPIC_API_KEY
echo
export ANTHROPIC_API_KEY
```

### OAuth-аутентификация

```bash
cd rust
./target/debug/claw login
./target/debug/claw logout
```

## Часто используемые операционные команды

```bash
cd rust
./target/debug/claw status
./target/debug/claw sandbox
./target/debug/claw agents
./target/debug/claw mcp
./target/debug/claw skills
./target/debug/claw system-prompt --cwd .. --date 2026-04-04
```

## Управление сессиями

REPL-ходы сохраняются в `.claw/sessions/` внутри текущего workspace.

```bash
cd rust
./target/debug/claw --resume latest
./target/debug/claw --resume latest /status /diff
```

Полезные интерактивные команды: `/help`, `/status`, `/cost`, `/config`, `/session`, `/model`, `/permissions` и `/export`.

## Порядок разрешения конфигурационных файлов

Runtime загружает конфигурацию в таком порядке, причем более поздние записи переопределяют более ранние:

1. `~/.claw.json`
2. `~/.config/claw/settings.json`
3. `<repo>/.claw.json`
4. `<repo>/.claw/settings.json`
5. `<repo>/.claw/settings.local.json`

## Стенд паритета на mock-сервисе

Workspace включает детерминированный mock-сервис, совместимый с Anthropic, и parity harness.

```bash
cd rust
./scripts/run_mock_parity_harness.sh
```

Ручной запуск mock-сервиса:

```bash
cd rust
cargo run -p mock-anthropic-service -- --bind 127.0.0.1:0
```

## Проверка

```bash
cd rust
cargo test --workspace
```

## Обзор workspace

Текущие Rust crate’ы:

- `api`
- `commands`
- `compat-harness`
- `mock-anthropic-service`
- `plugins`
- `runtime`
- `rusty-claude-cli`
- `telemetry`
- `tools`
