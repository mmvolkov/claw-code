# 🦞 Claw Code — реализация на Rust

Высокопроизводительное Rust-переписывание CLI agent harness под названием Claw Code. Проект строится ради скорости, безопасности и нативного выполнения инструментов.

Практическое руководство с готовыми примерами находится в [`../USAGE.md`](../USAGE.md).  
Подробный runbook по Docker, web UI, режимам auth, конфигурации, сессиям и troubleshooting находится в [`../docs/SETUP_AND_OPERATIONS.md`](../docs/SETUP_AND_OPERATIONS.md).

## Быстрый старт

```bash
# Посмотреть доступные команды
cd rust/
cargo run -p rusty-claude-cli -- --help

# Собрать workspace
cargo build --workspace

# Запустить интерактивный REPL
cargo run -p rusty-claude-cli -- --model claude-opus-4-6

# Одноразовый prompt
cargo run -p rusty-claude-cli -- prompt "explain this codebase"

# Вывод JSON для автоматизации
cargo run -p rusty-claude-cli -- --output-format json prompt "summarize src/main.rs"
```

## Конфигурация

Задайте API credentials:

```bash
read -s ANTHROPIC_API_KEY
echo
export ANTHROPIC_API_KEY
# Или используйте прокси
export ANTHROPIC_BASE_URL="http://127.0.0.1:8080"
```

Или пройдите OAuth-аутентификацию, чтобы CLI сохранял credentials локально:

```bash
cargo run -p rusty-claude-cli -- login
```

Важное текущее ограничение:

- сохраненный Claude OAuth корректно сохраняется и загружается
- для прямого inference в стандартный Anthropic Messages API все еще требуется `ANTHROPIC_API_KEY`
- OAuth-only transport для inference в этом репозитории пока не реализован

## Стенд паритета на mock-сервисе

Workspace теперь включает детерминированный mock-сервис, совместимый с Anthropic, и CLI-harness в чистом окружении для end-to-end parity checks.

```bash
cd rust/

# Запустить scripted harness в чистом окружении
./scripts/run_mock_parity_harness.sh

# Или поднять mock-сервис вручную для ad hoc CLI-запусков
cargo run -p mock-anthropic-service -- --bind 127.0.0.1:0
```

Покрытие harness:

- `streaming_text`
- `read_file_roundtrip`
- `grep_chunk_assembly`
- `write_file_allowed`
- `write_file_denied`
- `multi_tool_turn_roundtrip`
- `bash_stdout_roundtrip`
- `bash_permission_prompt_approved`
- `bash_permission_prompt_denied`
- `plugin_tool_roundtrip`

Основные артефакты:

- `crates/mock-anthropic-service/` — переиспользуемый mock Anthropic-compatible service
- `crates/rusty-claude-cli/tests/mock_parity_harness.rs` — CLI-harness в чистом окружении
- `scripts/run_mock_parity_harness.sh` — воспроизводимая обертка
- `scripts/run_mock_parity_diff.py` — runner для scenario checklist + PARITY mapping
- `mock_parity_scenarios.json` — manifest соответствия scenario-to-PARITY

## Возможности

| Возможность | Статус |
|---------|--------|
| Anthropic API + streaming | ✅ |
| OAuth login/logout | ✅ |
| Интерактивный REPL (`rustyline`) | ✅ |
| Система инструментов (`bash`, `read`, `write`, `edit`, `grep`, `glob`) | ✅ |
| Web-инструменты (search, fetch) | ✅ |
| Оркестрация sub-agent | ✅ |
| Отслеживание todo | ✅ |
| Редактирование notebook | ✅ |
| `CLAUDE.md` / project memory | ✅ |
| Иерархия config-файлов (`.claude.json`) | ✅ |
| Система прав | ✅ |
| Жизненный цикл MCP-серверов | ✅ |
| Сохранение сессий + resume | ✅ |
| Extended thinking (`thinking` blocks) | ✅ |
| Учёт стоимости и usage | ✅ |
| Git-интеграция | ✅ |
| Рендеринг Markdown в терминале (ANSI) | ✅ |
| Alias моделей (`opus` / `sonnet` / `haiku`) | ✅ |
| Slash-команды (`/status`, `/compact`, `/clear` и т.д.) | ✅ |
| Hooks (`PreToolUse` / `PostToolUse`) | 🔧 Только конфигурация |
| Plugin-система | 📋 Планируется |
| Реестр skills | 📋 Планируется |

## Псевдонимы моделей

Короткие имена разрешаются в актуальные версии моделей:

| Alias | Разворачивается в |
|-------|------------|
| `opus` | `claude-opus-4-6` |
| `sonnet` | `claude-sonnet-4-6` |
| `haiku` | `claude-haiku-4-5-20251213` |

## Флаги CLI

```
claw [OPTIONS] [COMMAND]

Options:
  --model MODEL                    Переопределить активную модель
  --dangerously-skip-permissions   Пропустить все проверки прав
  --permission-mode MODE           Установить read-only, workspace-write или danger-full-access
  --allowedTools TOOLS             Ограничить доступные инструменты
  --output-format FORMAT           Формат вывода для неинтерактивного режима (text или json)
  --resume SESSION                 Повторно открыть сохраненную сессию или инспектировать ее slash-командами
  --version, -V                    Локально вывести версию и build information

Commands:
  prompt <text>      Одноразовый prompt (неинтерактивный режим)
  login              Аутентификация через OAuth
  logout             Очистить сохраненные credentials
  init               Инициализировать project config
  status             Показать текущий snapshot статуса workspace
  sandbox            Показать текущий snapshot sandbox-изоляции
  agents             Показать определения агентов
  mcp                Показать настроенные MCP-серверы
  skills             Показать установленные skills
  system-prompt      Вывести собранный system prompt
```

Актуальный канонический help-текст смотрите через `cargo run -p rusty-claude-cli -- --help`.

## Слэш-команды в REPL

Tab completion разворачивает slash-команды, alias моделей, режимы прав и недавние session ID.

| Команда | Описание |
|---------|-------------|
| `/help` | Показать помощь |
| `/status` | Показать статус сессии (модель, токены, стоимость) |
| `/cost` | Показать разбивку стоимости |
| `/compact` | Уплотнить историю диалога |
| `/clear` | Очистить диалог |
| `/model [name]` | Показать или переключить модель |
| `/permissions` | Показать или переключить режим прав |
| `/config [section]` | Показать config (`env`, `hooks`, `model`) |
| `/memory` | Показать содержимое `CLAUDE.md` |
| `/diff` | Показать git diff |
| `/export [path]` | Экспортировать диалог |
| `/resume [id]` | Возобновить сохраненный диалог |
| `/session [id]` | Вернуться к предыдущей сессии |
| `/version` | Показать версию |

Примеры интерактивного использования, JSON-автоматизации, работы с сессиями, правами и mock parity harness находятся в [`../USAGE.md`](../USAGE.md).

## Структура workspace

```
rust/
├── Cargo.toml              # Корень workspace
├── Cargo.lock
└── crates/
    ├── api/                # Anthropic API client + SSE streaming
    ├── commands/           # Общий реестр slash-команд
    ├── compat-harness/     # Harness для извлечения manifest из TS
    ├── mock-anthropic-service/ # Детерминированный локальный Anthropic-compatible mock
    ├── plugins/            # Реестр plugins и примитивы hook-интеграции
    ├── runtime/            # Сессии, config, права, MCP, prompts
    ├── rusty-claude-cli/   # Основной CLI-бинарник (`claw`)
    ├── telemetry/          # Tracing и типы usage telemetry
    └── tools/              # Встроенные реализации инструментов
```

### Ответственность crate’ов

- **api** — HTTP-клиент, SSE-парсер, типы запросов/ответов, auth (API key + OAuth bearer)
- **commands** — определения slash-команд и генерация help-текста
- **compat-harness** — извлечение manifest инструментов и prompt’ов из upstream TS-исходника
- **mock-anthropic-service** — детерминированный mock `/v1/messages` для parity-тестов CLI и локальных harness-запусков
- **plugins** — метаданные plugin’ов, реестры и поверхности интеграции hooks
- **runtime** — агентный цикл `ConversationRuntime`, иерархия `ConfigLoader`, сохранение `Session`, политика прав, MCP-клиент, сборка system prompt, учет usage
- **rusty-claude-cli** — REPL, одноразовый prompt, streaming display, рендеринг tool call, парсинг CLI-аргументов
- **telemetry** — trace events сессий и вспомогательные payload телеметрии
- **tools** — спецификации и выполнение инструментов: Bash, ReadFile, WriteFile, EditFile, GlobSearch, GrepSearch, WebSearch, WebFetch, Agent, TodoWrite, NotebookEdit, Skill, ToolSearch, REPL runtimes

## Статистика

- **~20K строк** Rust-кода
- **9 crate’ов** в workspace
- **Имя бинарника:** `claw`
- **Модель по умолчанию:** `claude-opus-4-6`
- **Права по умолчанию:** `danger-full-access`

## Лицензия

Смотрите корень репозитория.
