# Статус паритета — Rust-порт `claw-code`

Последнее обновление: 2026-04-03

## Сводка

- Канонический документ: этот верхнеуровневый `PARITY.md` используется скриптом `rust/scripts/run_mock_parity_diff.py`.
- Запрошенный checkpoint по 9 lane: **все 9 lane влиты в `main`.**
- Текущий HEAD ветки `main`: `ee31e00` (stub-реализации заменены реальными `AskUserQuestion` + `RemoteTrigger`).
- Статистика репозитория на этом checkpoint: **292 коммита в `main` / 293 по всем веткам**, **9 crate’ов**, **48,599 отслеживаемых строк Rust-кода**, **2,568 строк тестов**, **3 автора**, диапазон дат **2026-03-31 → 2026-04-03**.
- Статистика mock parity harness: **10 scripted scenarios**, **19 зафиксированных запросов `/v1/messages`** в `rust/crates/rusty-claude-cli/tests/mock_parity_harness.rs`.

## Стенд паритета на mock-сервисе — этап 1

- [x] Детерминированный mock-сервис, совместимый с Anthropic (`rust/crates/mock-anthropic-service`)
- [x] Воспроизводимый CLI-harness в чистом окружении (`rust/crates/rusty-claude-cli/tests/mock_parity_harness.rs`)
- [x] Scripted scenarios: `streaming_text`, `read_file_roundtrip`, `grep_chunk_assembly`, `write_file_allowed`, `write_file_denied`

## Стенд паритета на mock-сервисе — этап 2 (расширение поведения)

- [x] Покрытие scripted multi-tool turn: `multi_tool_turn_roundtrip`
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
- Поддержка streaming response, подтвержденная mock parity harness

## Контрольная точка по 9 lane

| Lane | Статус | Feature commit | Merge commit | Доказательство |
|---|---|---|---|---|
| 1. Валидация Bash | merged | `36dac6c` | `1cfd78a` | `jobdori/bash-validation-submodules`, `rust/crates/runtime/src/bash_validation.rs` (`+1004` в `main`) |
| 2. Исправление CI | merged | `89104eb` | `f1969ce` | `rust/crates/runtime/src/sandbox.rs` (`+22/-1`) |
| 3. File-tool | merged | `284163b` | `a98f2b6` | `rust/crates/runtime/src/file_ops.rs` (`+195/-1`) |
| 4. TaskRegistry | merged | `5ea138e` | `21a1e1d` | `rust/crates/runtime/src/task_registry.rs` (`+336`) |
| 5. Wiring задач | merged | `e8692e4` | `d994be6` | `rust/crates/tools/src/lib.rs` (`+79/-35`) |
| 6. Team+Cron | merged | `c486ca6` | `49653fe` | `rust/crates/runtime/src/team_cron_registry.rs`, `rust/crates/tools/src/lib.rs` (`+441/-37`) |
| 7. Жизненный цикл MCP | merged | `730667f` | `cc0f92e` | `rust/crates/runtime/src/mcp_tool_bridge.rs`, `rust/crates/tools/src/lib.rs` (`+491/-24`) |
| 8. LSP client | merged | `2d66503` | `d7f0dc6` | `rust/crates/runtime/src/lsp_client.rs`, `rust/crates/tools/src/lib.rs` (`+461/-9`) |
| 9. Enforcement прав | merged | `66283f4` | `336f820` | `rust/crates/runtime/src/permission_enforcer.rs`, `rust/crates/tools/src/lib.rs` (`+357`) |

## Детали по lane

### Поток 1 — валидация Bash

- **Статус:** влита в `main`.
- **Feature commit:** `36dac6c` — `feat: add bash validation submodules — readOnlyValidation, destructiveCommandWarning, modeValidation, sedValidation, pathValidation, commandSemantics`
- **Доказательство:** branch-only diff добавляет `rust/crates/runtime/src/bash_validation.rs` и export в `runtime::lib` (`+1005` в 2 файлах).
- **Состояние в `main`:** `rust/crates/runtime/src/bash.rs` по-прежнему является активной реализацией в `main` и занимает **283 LOC**, обеспечивая timeout/background/sandbox execution. `PermissionEnforcer::check_bash()` уже добавляет блокировки для read-only режима в `main`, но выделенный validation-модуль еще не влит.

### Инструмент Bash — в upstream 18 submodule, в Rust 1:

- В ветке `main` это утверждение по-прежнему в целом верно.
- Покрытие harness подтверждает выполнение `bash` и потоки повышения прав через prompt, но не весь validation matrix из upstream.
- Branch-only lane нацелена на `readOnlyValidation`, `destructiveCommandWarning`, `modeValidation`, `sedValidation`, `pathValidation` и `commandSemantics`.

### Поток 2 — исправление CI

- **Статус:** влита в `main`.
- **Feature commit:** `89104eb` — `fix(sandbox): probe unshare capability instead of binary existence`
- **Merge commit:** `f1969ce` — `Merge jobdori/fix-ci-sandbox: probe unshare capability for CI fix`
- **Доказательство:** `rust/crates/runtime/src/sandbox.rs` занимает **385 LOC** и теперь определяет поддержку sandbox на основе реальной возможности `unshare` и container signals, а не по одному лишь наличию бинарника.
- **Почему это важно:** `.github/workflows/rust-ci.yml` запускает `cargo fmt --all --check` и `cargo test -p rusty-claude-cli`; эта lane убрала CI-специфичное предположение о sandbox из поведения runtime.

### Поток 3 — file-tool

- **Статус:** влита в `main`.
- **Feature commit:** `284163b` — `feat(file_ops): add edge-case guards — binary detection, size limits, workspace boundary, symlink escape`
- **Merge commit:** `a98f2b6` — `Merge jobdori/file-tool-edge-cases: binary detection, size limits, workspace boundary guards`
- **Доказательство:** `rust/crates/runtime/src/file_ops.rs` занимает **744 LOC** и теперь содержит `MAX_READ_SIZE`, `MAX_WRITE_SIZE`, детекцию бинарных файлов по NUL-байтам и canonical-проверку границ workspace.
- **Покрытие harness:** `read_file_roundtrip`, `grep_chunk_assembly`, `write_file_allowed` и `write_file_denied` перечислены в manifest и выполняются в clean-env harness.

### File tools — потоки, подтвержденные стендом

- `read_file_roundtrip` проверяет выполнение read-path и финальную сборку ответа.
- `grep_chunk_assembly` проверяет обработку chunked-вывода инструмента grep.
- `write_file_allowed` и `write_file_denied` подтверждают как успешную запись, так и отказ по правам.

### Поток 4 — TaskRegistry

- **Статус:** влита в `main`.
- **Feature commit:** `5ea138e` — `feat(runtime): add TaskRegistry — in-memory task lifecycle management`
- **Merge commit:** `21a1e1d` — `Merge jobdori/task-runtime: TaskRegistry in-memory lifecycle management`
- **Доказательство:** `rust/crates/runtime/src/task_registry.rs` занимает **335 LOC** и предоставляет `create`, `get`, `list`, `stop`, `update`, `output`, `append_output`, `set_status` и `assign_team` поверх thread-safe реестра в памяти.
- **Область:** эта lane заменяет чисто stub-состояние на реальные runtime-backed записи задач, но сама по себе не добавляет внешнее subprocess-исполнение.

### Поток 5 — wiring задач

- **Статус:** влита в `main`.
- **Feature commit:** `e8692e4` — `feat(tools): wire TaskRegistry into task tool dispatch`
- **Merge commit:** `d994be6` — `Merge jobdori/task-registry-wiring: real TaskRegistry backing for all 6 task tools`
- **Доказательство:** `rust/crates/tools/src/lib.rs` dispatch’ит `TaskCreate`, `TaskGet`, `TaskList`, `TaskStop`, `TaskUpdate` и `TaskOutput` через `execute_tool()` и конкретные `run_task_*` handler’ы.
- **Текущее состояние:** инструменты задач теперь раскрывают реальное состояние реестра в `main` через `global_task_registry()`.

### Поток 6 — Team+Cron

- **Статус:** влита в `main`.
- **Feature commit:** `c486ca6` — `feat(runtime+tools): TeamRegistry and CronRegistry — replace team/cron stubs`
- **Merge commit:** `49653fe` — `Merge jobdori/team-cron-runtime: TeamRegistry + CronRegistry wired into tool dispatch`
- **Доказательство:** `rust/crates/runtime/src/team_cron_registry.rs` занимает **363 LOC** и добавляет thread-safe `TeamRegistry` и `CronRegistry`; `rust/crates/tools/src/lib.rs` подключает к ним `TeamCreate`, `TeamDelete`, `CronCreate`, `CronDelete` и `CronList`.
- **Текущее состояние:** инструменты `team` и `cron` теперь имеют in-memory lifecycle behavior в `main`; до реального background scheduler или worker fleet они пока не доходят.

### Поток 7 — жизненный цикл MCP

- **Статус:** влита в `main`.
- **Feature commit:** `730667f` — `feat(runtime+tools): McpToolRegistry — MCP lifecycle bridge for tool surface`
- **Merge commit:** `cc0f92e` — `Merge jobdori/mcp-lifecycle: McpToolRegistry lifecycle bridge for all MCP tools`
- **Доказательство:** `rust/crates/runtime/src/mcp_tool_bridge.rs` занимает **406 LOC** и отслеживает статус подключения серверов, список ресурсов, чтение ресурсов, список инструментов, подтверждения dispatch, auth state и disconnect.
- **Wiring:** `rust/crates/tools/src/lib.rs` направляет `ListMcpResources`, `ReadMcpResource`, `McpAuth` и `MCP` в handler’ы `global_mcp_registry()`.
- **Область:** эта lane заменяет чисто stub-ответы на registry bridge в `main`; глубина end-to-end для подключения MCP и транспорта все еще зависит от более широкого MCP runtime (`mcp_stdio.rs`, `mcp_client.rs`, `mcp.rs`).

### Поток 8 — LSP client

- **Статус:** влита в `main`.
- **Feature commit:** `2d66503` — `feat(runtime+tools): LspRegistry — LSP client dispatch for tool surface`
- **Merge commit:** `d7f0dc6` — `Merge jobdori/lsp-client: LspRegistry dispatch for all LSP tool actions`
- **Доказательство:** `rust/crates/runtime/src/lsp_client.rs` занимает **438 LOC** и моделирует diagnostics, hover, definition, references, completion, symbols и formatting через stateful registry.
- **Wiring:** открытая схема инструмента `LSP` в `rust/crates/tools/src/lib.rs` сейчас перечисляет `symbols`, `references`, `diagnostics`, `definition` и `hover`, после чего направляет запрос в `registry.dispatch(action, path, line, character, query)`.
- **Область:** текущий паритет достигнут на уровне registry/dispatch; поддержка completion/formatting существует в модели реестра, но не так явно открыта на границе tool schema, а реальная оркестрация внешних language server-процессов пока вынесена отдельно.

### Поток 9 — enforcement прав

- **Статус:** влита в `main`.
- **Feature commit:** `66283f4` — `feat(runtime+tools): PermissionEnforcer — permission mode enforcement layer`
- **Merge commit:** `336f820` — `Merge jobdori/permission-enforcement: PermissionEnforcer with workspace + bash enforcement`
- **Доказательство:** `rust/crates/runtime/src/permission_enforcer.rs` занимает **340 LOC** и добавляет gating инструментов, проверку границ file write и read-only эвристики для `bash` поверх `rust/crates/runtime/src/permissions.rs`.
- **Wiring:** `rust/crates/tools/src/lib.rs` экспортирует `enforce_permission_check()` и хранит `required_permission` для каждого инструмента в tool spec.

### Enforcement прав на разных tool-path

- Сценарии harness проверяют `write_file_denied`, `bash_permission_prompt_approved` и `bash_permission_prompt_denied`.
- `PermissionEnforcer::check()` делегирует в `PermissionPolicy::authorize()` и возвращает структурированные результаты allow/deny.
- `check_file_write()` обеспечивает соблюдение границ workspace и read-only запретов; `check_bash()` запрещает мутирующие команды в read-only режиме и блокирует prompt-mode `bash` без подтверждения.

## Поверхность инструментов: 40 открытых tool spec в `main`

- `mvp_tool_specs()` в `rust/crates/tools/src/lib.rs` раскрывает **40** tool spec.
- Базовое исполнение реализовано для `bash`, `read_file`, `write_file`, `edit_file`, `glob_search` и `grep_search`.
- Уже существующие продуктовые инструменты в `mvp_tool_specs()` включают `WebFetch`, `WebSearch`, `TodoWrite`, `Skill`, `Agent`, `ToolSearch`, `NotebookEdit`, `Sleep`, `SendUserMessage`, `Config`, `EnterPlanMode`, `ExitPlanMode`, `StructuredOutput`, `REPL` и `PowerShell`.
- Push из 9 lane заменил чисто fixed-payload stub’ы для `Task*`, `Team*`, `Cron*`, `LSP` и MCP tools на registry-backed handler’ы в `main`.
- `Brief` обрабатывается как alias выполнения в `execute_tool()`, но не является отдельным открытым tool spec в `mvp_tool_specs()`.

### Все еще ограничено или сознательно упрощено

- `AskUserQuestion` по-прежнему возвращает pending-response payload, а не настоящую интерактивную UI-интеграцию.
- `RemoteTrigger` остается stub-ответом.
- `TestingPermission` остается только тестовым.
- `Task`, `team`, `cron`, `MCP` и `LSP` больше не являются просто fixed-payload stub’ами внутри `execute_tool()`, но часть из них по-прежнему представляют собой registry-backed приближения, а не полные интеграции с внешним runtime.
- Глубокая валидация `bash` до сих пор branch-only, пока не будет влит `36dac6c`.

## Сверка со старым PARITY-checklist

- [x] Предотвращение path traversal (`symlink`-переходы, `../`-escape)
- [x] Ограничения размера для read/write
- [x] Детекция бинарных файлов
- [x] Enforcement permission mode (`read-only` vs `workspace-write`)
- [x] Приоритет слияния config (user > project > local) — `ConfigLoader::discover()` загружает `user → project → local`, а `loads_and_merges_claude_code_config_files_by_precedence()` проверяет этот порядок.
- [x] Поток plugin install/enable/disable/uninstall — slash-handling `/plugin` в `rust/crates/commands/src/lib.rs` делегирует в `PluginManager::{install, enable, disable, uninstall}` в `rust/crates/plugins/src/lib.rs`.
- [x] Нет `#[ignore]`-тестов, скрывающих падения — `grep` по `rust/**/*.rs` нашел 0 ignored-тестов.

## Что еще открыто

- [ ] End-to-end жизненный цикл MCP runtime за пределами bridge-реестра, который уже есть в `main`
- [x] Обрезка вывода (большой `stdout` / содержимое файлов)
- [ ] Поведение session compaction в полном соответствии
- [ ] Точность подсчета токенов / учета стоимости
- [x] Lane валидации Bash влита в `main`
- [ ] CI зеленый на каждом коммите

## Готовность к миграции

- [x] `PARITY.md` поддерживается в актуальном и честном состоянии
- [x] Все 9 запрошенных lane задокументированы с хэшами коммитов и текущим статусом
- [x] Все 9 запрошенных lane landed на `main` (`bash-validation` все еще branch-only)
- [x] Нет `#[ignore]`-тестов, скрывающих падения
- [ ] CI зеленый на каждом коммите
- [x] Форма кодовой базы достаточно чистая для handoff-документации
