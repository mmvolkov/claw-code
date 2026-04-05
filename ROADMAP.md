# ROADMAP.md

# Дорожная карта Clawable Coding Harness

## Цель

Превратить `claw-code` в максимально **clawable** coding harness:
- без предположений о человеке как основном пользователе терминала
- без хрупких зависимостей от момента prompt-injection
- без непрозрачного состояния сессий
- без скрытых падений plugin’ов или MCP
- без ручного нянченья при типовых recovery-сценариях

Эта дорожная карта исходит из того, что основные пользователи — это **claws, связанные через hooks, plugins, sessions и channel events**.

## Что значит «clawable»

Clawable harness:
- детерминированно стартует
- имеет машинно-читаемое состояние и режимы отказа
- умеет восстанавливаться без человека, следящего за терминалом
- знает о branch/test/worktree-состоянии
- знает о жизненном цикле plugin’ов и MCP
- строится вокруг событий, а не логов
- способен автономно выполнять следующий шаг

## Текущие болевые точки

### 1. Старт сессии хрупкий
- trust-prompt может заблокировать запуск TUI
- prompt может уйти в shell вместо coding-агента
- «session exists» не означает «session is ready»

### 2. Истина размазана по слоям
- состояние tmux
- поток событий clawhip
- состояние git/worktree
- состояние тестов
- состояние gateway/plugin/MCP runtime

### 3. События слишком похожи на логи
- claws сейчас вынуждены слишком многое выводить из шумного текста
- важные состояния не нормализованы в машинно-читаемые события

### 4. Recovery loops слишком ручные
- перезапуск worker
- принятие trust-prompt
- повторная инъекция prompt
- обнаружение stale-branch
- повторная попытка failed startup
- ручная классификация infra-ошибок и кодовых ошибок

### 5. Свежесть веток контролируется недостаточно
- side-branch могут не содержать уже влитые исправления из `main`
- массовые падения тестов могут быть шумом stale-branch, а не настоящими регрессиями

### 6. Ошибки plugin/MCP классифицируются слабо
- сбои старта, handshake-ошибки, ошибки конфигурации, частичный старт и degraded mode раскрываются недостаточно чисто

### 7. Человеческий UX все еще протекает в claw-workflow
- слишком многое зависит от поведения терминала/TUI вместо явных переходов состояния агента и control API

## Продуктовые принципы

1. **Сначала машина состояний** — у каждого worker есть явные lifecycle-state.
2. **События важнее scraped prose** — вывод в канал должен порождаться из типизированных событий.
3. **Recovery до escalation** — известные режимы отказа должны один раз лечиться автоматически до просьбы о помощи.
4. **Свежесть ветки до поиска виноватого** — сначала проверяем stale branch, и только потом считаем красные тесты новой регрессией.
5. **Частичный успех — first-class** — например, старт MCP может быть успешен для части серверов и провален для других, с формализованной degraded-mode отчетностью.
6. **Терминал — это транспорт, а не истина** — tmux/TUI могут остаться деталями реализации, но orchestration-state должно жить уровнем выше.
7. **Политика должна исполняться** — merge, retry, rebase, cleanup stale-state и escalation-правила должны быть enforce-имы машиной.

## Дорожная карта

## Фаза 1 — надежный старт worker’ов

### 1. Ready-handshake lifecycle для coding-worker’ов
Добавить явные состояния:
- `spawning`
- `trust_required`
- `ready_for_prompt`
- `prompt_accepted`
- `running`
- `blocked`
- `finished`
- `failed`

Acceptance:
- prompt никогда не отправляется раньше `ready_for_prompt`
- состояние trust-prompt можно обнаружить и эмитить как событие
- ошибочная доставка в shell становится обнаруживаемым first-class failure-state

### 2. Resolver trust-prompt
Добавить allowlist-поведение для auto-trust в известных repo/worktree.

Acceptance:
- доверенные repo автоматически снимают trust-prompt
- эмитятся события `trust_required` и `trust_resolved`
- repo вне allowlist по-прежнему остаются под защитой

### 3. Structured session control API
Дать машинный control plane поверх tmux:
- создать worker
- дождаться готовности
- отправить задачу
- получить состояние
- получить последнюю ошибку
- перезапустить worker
- завершить worker

Acceptance:
- claw может управлять coding-worker без raw `send-keys` как основной control-plane

## Фаза 2 — event-native интеграция с clawhip

### 4. Каноническая схема lane-событий
Определить типизированные события вроде:
- `lane.started`
- `lane.ready`
- `lane.prompt_misdelivery`
- `lane.blocked`
- `lane.red`
- `lane.green`
- `lane.commit.created`
- `lane.pr.opened`
- `lane.merge.ready`
- `lane.finished`
- `lane.failed`
- `branch.stale_against_main`

Acceptance:
- clawhip потребляет типизированные lane-события
- сводки для Discord рендерятся из структурированных событий, а не только из pane-scraping

### 5. Таксономия ошибок
Нормализовать классы отказов:
- `prompt_delivery`
- `trust_gate`
- `branch_divergence`
- `compile`
- `test`
- `plugin_startup`
- `mcp_startup`
- `mcp_handshake`
- `gateway_routing`
- `tool_runtime`
- `infra`

Acceptance:
- блокеры классифицируются машиной
- dashboards и retry policy могут ветвиться по типу ошибки

### 6. Сжатие до actionable summary
Свести шумный поток событий к:
- текущей фазе
- последнему успешному checkpoint
- текущему blocker’у
- рекомендованному следующему recovery-действию

Acceptance:
- обновления статуса в каналах остаются короткими и привязанными к машинному состоянию
- claws перестают выводить состояние из сырого build-spam

## Фаза 3 — awareness по веткам/тестам и auto-recovery

### 7. Обнаружение stale-branch до широких проверок
Перед широким прогоном тестов сравнивать текущую ветку с `main` и определять, не пропущены ли уже влитые исправления.

Acceptance:
- эмитится `branch.stale_against_main`
- предлагается или автоматически выполняется rebase/merge-forward согласно policy
- stale-branch failure не классифицируются ошибочно как новые регрессии

### 8. Recovery recipes для типовых ошибок
Закодировать известные автоматические recovery-сценарии для:
- unresolved trust-prompt
- prompt, доставленного в shell
- stale branch
- compile red после cross-crate refactor
- handshake-failure при старте MCP
- частичного старта plugin’ов

Acceptance:
- до escalation выполняется одна автоматическая попытка recovery
- сама попытка recovery тоже эмитится как структурированные данные события

### 9. Контракт на «зеленость»
Worker’ы должны различать:
- targeted tests green
- package green
- workspace green
- merge-ready green

Acceptance:
- больше не будет двусмысленного «tests passed»
- merge policy сможет требовать корректный уровень green для данного типа lane

## Фаза 4 — исполнение задач в модели claws-first

### 10. Typed task packet format
Определить структурированный task packet с полями вроде:
- objective
- scope
- repo/worktree
- branch policy
- acceptance tests
- commit policy
- reporting contract
- escalation policy

Acceptance:
- claws смогут dispatch’ить работу, не полагаясь только на длинные natural-language prompt’ы
- task packet можно безопасно логировать, повторять и трансформировать

### 11. Policy engine для автономного кодинга
Закодировать правила автоматизации, например:
- если green + scoped diff + review passed -> merge в `dev`
- если stale branch -> merge-forward до широких тестов
- если startup blocked -> один recovery, потом escalation
- если lane completed -> emit closeout и cleanup session

Acceptance:
- доктрина переезжает из чат-инструкций в исполнимые правила

### 12. Claw-native dashboards / lane board
Открыть машинно-читаемую доску со следующими данными:
- repo
- активные claws
- worktree
- свежесть веток
- red/green-состояние
- текущий blocker
- готовность к merge
- последнее meaningful event

Acceptance:
- claws могут напрямую запрашивать статус
- человеко-ориентированные представления становятся лишь слоем рендеринга, а не источником истины

## Фаза 5 — зрелость жизненного цикла plugin’ов и MCP

### 13. First-class контракт жизненного цикла plugin/MCP
Каждая plugin/MCP-интеграция должна раскрывать:
- контракт валидации конфигурации
- startup healthcheck
- результат discovery
- поведение в degraded mode
- shutdown/cleanup contract

Acceptance:
- partial startup и per-server failures сообщаются структурированно
- успешные серверы остаются пригодными к использованию даже при падении одного из серверов

### 14. Полный паритет жизненного цикла MCP
Закрыть разрывы в:
- загрузке конфигурации
- регистрации серверов
- spawn/connect
- initialize-handshake
- discovery инструментов/ресурсов
- invocation path
- surfaced errors
- shutdown/cleanup

Acceptance:
- parity harness и runtime tests покрывают healthy и degraded startup cases
- сломанные серверы раскрываются как структурированные ошибки, а не непрозрачные предупреждения

## Непосредственный backlog (из реальных текущих проблем)

Порядок приоритета: P0 = блокирует CI/green-state, P1 = блокирует wiring интеграций, P2 = усиливает clawability, P3 = повышает эффективность swarm.

**P0 — исправить в первую очередь (надежность CI)**
1. Изолировать тесты `render_diff_report` в tmpdir — сейчас они flaky под `cargo test --workspace`, читают реальное состояние working tree и ломают CI во время активных операций с worktree
2. Расширить GitHub CI с покрытия одного crate до проверки уровня workspace — текущий `rust-ci.yml` запускает `cargo fmt` и `cargo test -p rusty-claude-cli`, но не охватывает более широкий `cargo test --workspace`, который уже проходит локально
3. Добавить release-grade workflow для бинарников — в репозитории есть Rust CLI и намерение на релиз, но нет GitHub Actions-пути, который собирал бы tagged artifacts / проверял упаковку релиза до шага публикации
4. Добавить container-first docs для тестирования и запуска — runtime умеет определять Docker/Podman/container state, но документация не показывает канонический контейнерный workflow для `cargo test --workspace`, запуска бинарников или работы с bind-mounted repo
5. Вынести `doctor` / preflight diagnostics в onboarding docs и help — CLI уже содержит команды диагностики setup и preflight по веткам, но в README/USAGE они недостаточно заметны, поэтому новые пользователи по-прежнему задают ручные setup-вопросы вместо запуска встроенной проверки здоровья
6. Добавить residue-check для брендинга и source-of-truth в документации — после миграции репозитория старые названия организаций могут оставаться в badges, star-history URL и скопированных snippets; нужен consistency-pass или CI-lint, который будет автоматически ловить устаревший брендинг
7. Согласовать narrative README с текущей реальностью репозитория — верхнеуровневая документация теперь говорит, что активный workspace — Rust, но в более поздних секциях репозиторий все еще описывается как Python-first; пользователь не должен гадать, какая реализация каноническая
8. Устранить warning-spam из first-run help/build path — `cargo run -p rusty-claude-cli -- --help` сейчас печатает стену compile warning до реального help-текста, что портит first-touch UX и скрывает продуктовую поверхность за несвязанным шумом

**P1 — далее (wiring интеграций, разблокировка проверок)**
2. Добавить cross-module integration tests — **готово**: 12 integration-тестов покрывают worker→recovery→policy, stale_branch→policy, green_contract→policy и reconciliation flows
3. Подключить emitter завершения lane — **готово**: модуль `lane_completion` с `detect_lane_completion()` автоматически выставляет `LaneContext::completed` по комбинации session-finished + tests-green + push-complete → policy closeout
4. Подключить `SummaryCompressor` в pipeline lane-событий — **готово**: `compress_summary_text()` подается в detail field `LaneEvent::Finished` в `tools/src/lib.rs`

**P2 — hardening clawability (исходный backlog)**
5. Handshake готовности worker’ов + trust resolution — **готово**: машина состояний `WorkerStatus` с жизненным циклом `Spawning` → `TrustRequired` → `ReadyForPrompt` → `PromptAccepted` → `Running`, а также `trust_auto_resolve` + `trust_gate_cleared`
6. Обнаружение и recovery prompt misdelivery — **готово**: счетчик `prompt_delivery_attempts`, обнаружение события `PromptMisdelivery`, ветка recovery `auto_recover_prompt_misdelivery` + `replay_prompt`
7. Каноническая схема lane-событий в clawhip — **готово**: enum `LaneEvent` с вариантами `Started/Blocked/Failed/Finished`, типизированный конструктор `LaneEvent::new()`, интеграция в `tools/src/lib.rs`
8. Таксономия ошибок + нормализация blocker’ов — **готово**: enum `WorkerFailureKind` (`TrustGate/PromptDelivery/Protocol/Provider`), bridge `FailureScenario::from_worker_failure_kind()` к recovery recipes
9. Обнаружение stale-branch до workspace-тестов — **готово**: модуль `stale_branch.rs` с определением свежести, метриками behind/ahead и интеграцией с policy
10. Структурированная отчетность по degraded-startup для MCP — **готово**: отчетность `McpManager` по degraded startup (+183 строки в `mcp_stdio.rs`), классификация failed server (startup/handshake/config/partial), структурированные `failed_servers` + `recovery_recommendations` в tool output
11. Structured task packet format — **готово**: модуль `task_packet.rs` со struct `TaskPacket`, валидацией, сериализацией, разрешением `TaskScope` (workspace/module/single-file/custom), интеграция в `tools/src/lib.rs`
12. Lane board / machine-readable status API — **готово**: hardening завершения lane + автоопределение `LaneContext::completed` + отчетность MCP degraded mode открывают машинно-читаемое состояние
13. **Классификация ошибок завершения сессии** — **готово**: landed `WorkerFailureKind::Provider` + `observe_completion()` + bridge к recovery recipes
14. **Пробел в валидации merge config** — **готово**: в `config.rs` добавлена hook-validation до deep-merge (+56 строк), malformed entries теперь падают с контекстом пути к источнику, а не общей merged parse error
15. **Flaky-тест discovery для MCP manager** — `manager_discovery_report_keeps_healthy_servers_when_one_server_fails` имеет периодические timing-problem в CI; временно помечен `ignored`, нужно отдельное исправление первопричины

**P3 — эффективность swarm**
13. Протокол branch-lock для swarm — определять коллизии same-module/same-branch до того, как параллельные worker’ы разъедутся в дублирующие реализации
14. Commit provenance / worktree-aware push events — эмитить ветку, worktree, superseded-by и каноническую commit lineage, чтобы параллельные сессии перестали порождать похожие дублирующиеся push-summary

## Рекомендуемое разбиение по сессиям

### Сессия A — протокол старта worker’ов
Фокус:
- обнаружение trust-prompt
- handshake `ready-for-prompt`
- обнаружение prompt misdelivery

### Сессия B — lane-события clawhip
Фокус:
- каноническая схема lane-событий
- таксономия ошибок
- summary compression

### Сессия C — branch/test intelligence
Фокус:
- определение stale-branch
- green-level contract
- recovery recipes

### Сессия D — hardening жизненного цикла MCP
Фокус:
- надежность startup/handshake
- структурированная отчетность по failed server
- runtime-поведение в degraded mode
- покрытие lifecycle tests/harness

### Сессия E — typed task packets + policy engine
Фокус:
- структурированный формат задач
- retry/merge/escalation rules
- поведение автономного закрытия lane

## Критерии успеха для MVP

Можно считать `claw-code` заметно более clawable, когда:
- claw может запустить worker и с уверенностью знать, когда тот готов
- claws больше не печатают задачи случайно в shell
- stale-branch ошибки определяются до того, как на них тратится время отладки
- clawhip сообщает машинные состояния, а не только prose из tmux
- ошибки старта MCP/plugin классифицируются и раскрываются чисто
- coding lane может самостоятельно восстановиться после типовых startup- и branch-проблем без человеческого babysitting

## Короткая версия

`claw-code` должен эволюционировать из:
- CLI, которым также может управлять человек

в:
- **claw-native execution runtime**
- **event-native orchestration substrate**
- **plugin/hook-first autonomous coding harness**
