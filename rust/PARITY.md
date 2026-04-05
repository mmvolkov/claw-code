# Статус паритета — Rust-порт `claw-code`

Последнее обновление: 2026-04-03

## Стенд паритета на mock-сервисе — этап 1

- [x] Детерминированный mock-сервис, совместимый с Anthropic (`rust/crates/mock-anthropic-service`)
- [x] Воспроизводимый CLI-harness в чистом окружении (`rust/crates/rusty-claude-cli/tests/mock_parity_harness.rs`)
- [x] Scripted scenarios: `streaming_text`, `read_file_roundtrip`, `grep_chunk_assembly`, `write_file_allowed`, `write_file_denied`

## Стенд паритета на mock-сервисе — этап 2 (расширение поведения)

- [x] Scripted-покрытие multi-tool turn: `multi_tool_turn_roundtrip`
- [x] Scripted-покрытие `bash`: `bash_stdout_roundtrip`
- [x] Scripted-покрытие permission prompt: `bash_permission_prompt_approved`, `bash_permission_prompt_denied`
- [x] Scripted-покрытие plugin-path: `plugin_tool_roundtrip`
- [x] Runner для behavioral diff/checklist: `rust/scripts/run_mock_parity_diff.py`

## Поведенческий checklist harness v2

Каноническая карта сценариев: `rust/mock_parity_scenarios.json`

- Ходы ассистента с несколькими инструментами
- Roundtrip для `bash`
- Enforcement прав на разных tool-path
- Путь выполнения plugin-tool
- File tools — потоки, подтвержденные harness

## Завершенная работа по behavioral parity

Хэши ниже взяты из `git log --oneline`. Подсчет строк в merge — из `git show --stat <merge>`.

| Lane | Статус | Feature commit | Merge commit | Diff stat |
|------|--------|----------------|--------------|-----------|
| Валидация Bash (9 submodule) | ✅ complete | `36dac6c` | — (`jobdori/bash-validation-submodules`) | `1005 insertions` |
| Исправление CI | ✅ complete | `89104eb` | `f1969ce` | `22 insertions, 1 deletion` |
| Edge-cases для File-tool | ✅ complete | `284163b` | `a98f2b6` | `195 insertions, 1 deletion` |
| TaskRegistry | ✅ complete | `5ea138e` | `21a1e1d` | `336 insertions` |
| Wiring task tool | ✅ complete | `e8692e4` | `d994be6` | `79 insertions, 35 deletions` |
| Runtime для team + cron | ✅ complete | `c486ca6` | `49653fe` | `441 insertions, 37 deletions` |
| Жизненный цикл MCP | ✅ complete | `730667f` | `cc0f92e` | `491 insertions, 24 deletions` |
| LSP client | ✅ complete | `2d66503` | `d7f0dc6` | `461 insertions, 9 deletions` |
| Enforcement прав | ✅ complete | `66283f4` | `336f820` | `357 insertions` |

## Поверхность инструментов: 40/40 (паритет по spec)

### Реальные реализации (behavioral parity разной глубины)

| Tool | Rust Impl | Поведенческие заметки |
|------|-----------|-----------------|
| **bash** | `runtime::bash` 283 LOC | выполнение subprocess, timeout, background, sandbox — **сильный паритет**. Все 9/9 запрошенных validation-submodule теперь учитываются как completed через `36dac6c`, а в `main` уже есть runtime-поддержка sandbox + permission enforcement |
| **read_file** | `runtime::file_ops` | чтение с offset/limit — **хороший паритет** |
| **write_file** | `runtime::file_ops` | создание/перезапись файлов — **хороший паритет** |
| **edit_file** | `runtime::file_ops` | замена строк old/new — **хороший паритет**. Недостающее: недавно добавлен `replace_all` |
| **glob_search** | `runtime::file_ops` | сопоставление glob-паттернов — **хороший паритет** |
| **grep_search** | `runtime::file_ops` | поиск в стиле `ripgrep` — **хороший паритет** |
| **WebFetch** | `tools` | загрузка URL + извлечение контента — **умеренный паритет** (нужно дополнительно сверить обрезку контента и redirect handling относительно upstream) |
| **WebSearch** | `tools` | выполнение поисковых запросов — **умеренный паритет** |
| **TodoWrite** | `tools` | сохранение todo/заметок — **умеренный паритет** |
| **Skill** | `tools` | discovery/install skills — **умеренный паритет** |
| **Agent** | `tools` | делегирование агентам — **умеренный паритет** |
| **TaskCreate** | `runtime::task_registry` + `tools` | создание задач в памяти, подключенное к tool dispatch — **хороший паритет** |
| **TaskGet** | `runtime::task_registry` + `tools` | lookup задач + payload с метаданными — **хороший паритет** |
| **TaskList** | `runtime::task_registry` + `tools` | listing задач через реестр — **хороший паритет** |
| **TaskStop** | `runtime::task_registry` + `tools` | обработка остановки и terminal state — **хороший паритет** |
| **TaskUpdate** | `runtime::task_registry` + `tools` | обновления сообщений через реестр — **хороший паритет** |
| **TaskOutput** | `runtime::task_registry` + `tools` | получение накопленного вывода — **хороший паритет** |
| **TeamCreate** | `runtime::team_cron_registry` + `tools` | жизненный цикл команд + назначение задач — **хороший паритет** |
| **TeamDelete** | `runtime::team_cron_registry` + `tools` | удаление команд — **хороший паритет** |
| **CronCreate** | `runtime::team_cron_registry` + `tools` | создание cron-записей — **хороший паритет** |
| **CronDelete** | `runtime::team_cron_registry` + `tools` | удаление cron-записей — **хороший паритет** |
| **CronList** | `runtime::team_cron_registry` + `tools` | listing cron через реестр — **хороший паритет** |
| **LSP** | `runtime::lsp_client` + `tools` | реестр + dispatch для diagnostics, hover, definition, references, completion, symbols, formatting — **хороший паритет** |
| **ListMcpResources** | `runtime::mcp_tool_bridge` + `tools` | listing ресурсов подключенных серверов — **хороший паритет** |
| **ReadMcpResource** | `runtime::mcp_tool_bridge` + `tools` | чтение ресурсов подключенных серверов — **хороший паритет** |
| **MCP** | `runtime::mcp_tool_bridge` + `tools` | stateful bridge для вызова MCP-инструментов — **хороший паритет** |
| **ToolSearch** | `tools` | discovery инструментов — **хороший паритет** |
| **NotebookEdit** | `tools` | редактирование ячеек Jupyter notebook — **умеренный паритет** |
| **Sleep** | `tools` | задержка выполнения — **хороший паритет** |
| **SendUserMessage/Brief** | `tools` | сообщение пользователю — **хороший паритет** |
| **Config** | `tools` | inspection конфигурации — **умеренный паритет** |
| **EnterPlanMode** | `tools` | переключение worktree в plan mode — **хороший паритет** |
| **ExitPlanMode** | `tools` | восстановление worktree из plan mode — **хороший паритет** |
| **StructuredOutput** | `tools` | passthrough JSON — **хороший паритет** |
| **REPL** | `tools` | выполнение кода через subprocess — **умеренный паритет** |
| **PowerShell** | `tools` | выполнение Windows PowerShell — **умеренный паритет** |

### Только stub’ы (паритет по surface, без поведения)

| Tool | Статус | Заметки |
|------|--------|-------|
| **AskUserQuestion** | stub | нужна живая интеграция с пользовательским I/O |
| **McpAuth** | stub | нужен полноценный auth UX сверх MCP lifecycle bridge |
| **RemoteTrigger** | stub | нужен HTTP-клиент |
| **TestingPermission** | stub | только для тестов, низкий приоритет |

## Слэш-команды: 67/141 upstream entries

- 27 исходных spec — все с реальными handler’ами
- 40 новых spec — parse + stub handler (`not yet implemented`)
- Оставшиеся ~74 upstream entry — это внутренние модули/диалоги/шаги, а не пользовательские `/commands`

### Поведенческие checkpoints по функциональности (завершенная работа + оставшиеся разрывы)

**Инструмент Bash — все 9/9 запрошенных validation submodule завершены:**
- [x] `sedValidation` — валидация команд `sed` перед выполнением
- [x] `pathValidation` — валидация путей к файлам в командах
- [x] `readOnlyValidation` — блокировка записи в read-only режиме
- [x] `destructiveCommandWarning` — предупреждение для `rm -rf` и подобных команд
- [x] `commandSemantics` — классификация намерения команды
- [x] `bashPermissions` — permission gating по типу команды
- [x] `bashSecurity` — security checks
- [x] `modeValidation` — валидация относительно текущего режима прав
- [x] `shouldUseSandbox` — логика принятия решения о sandbox

Примечание по harness: milestone 2 проверяет успешный `bash`, а также approve/deny-потоки для повышения прав в режиме `workspace-write`; выделенные validation-submodule landed в `36dac6c`, а runtime в `main` уже содержит sandbox + permission enforcement.

**File tools — завершенный checkpoint:**
- [x] Предотвращение path traversal (`symlink`, `../`-escape)
- [x] Ограничения размера для read/write
- [x] Детекция бинарных файлов
- [x] Enforcement permission mode (`read-only` vs `workspace-write`)

Примечание по harness: `read_file`, `grep_search`, allow/deny для `write_file` и сборка multi-tool в рамках одного turn теперь покрыты mock parity harness; edge-cases для файлов и permission enforcement landed в `a98f2b6` и `336f820`.

**Config/Plugin/MCP flows:**
- [x] Полный жизненный цикл MCP-сервера (connect, list tools, call tool, disconnect)
- [ ] Полный поток plugin install/enable/disable/uninstall
- [ ] Приоритет merge для config (user > project > local)

Примечание по harness: внешнее обнаружение и выполнение plugin’ов теперь покрыто через `plugin_tool_roundtrip`; MCP lifecycle landed в `cc0f92e`, а plugin lifecycle и приоритет merge конфигурации пока остаются открытыми.

## Разрывы поведения в runtime

- [x] Enforcement прав для всех инструментов (`read-only`, `workspace-write`, `danger-full-access`)
- [ ] Обрезка вывода (большой `stdout` / содержимое файлов)
- [ ] Поведение session compaction в полном соответствии
- [ ] Точность подсчета токенов / учета стоимости
- [x] Поддержка streaming response подтверждена mock parity harness

Примечание по harness: текущее покрытие уже включает отказ записи в файл, approve/deny для `bash` escalation и выполнение plugin’ов с `workspace-write`; permission enforcement landed в `336f820`.

## Готовность к миграции

- [x] `PARITY.md` поддерживается в актуальном и честном состоянии
- [ ] Нет `#[ignore]`-тестов, скрывающих падения (допускается только 1: `live_stream_smoke_test`)
- [ ] CI зеленый на каждом коммите
- [ ] Форма кодовой базы достаточно чистая для handoff
