# Стенд паритета на mock-сервисе для LLM

На этом этапе добавлены детерминированный mock-сервис, совместимый с Anthropic, и воспроизводимый CLI-harness для Rust-бинарника `claw`.

## Артефакты

- `crates/mock-anthropic-service/` — mock-сервис `/v1/messages`
- `crates/rusty-claude-cli/tests/mock_parity_harness.rs` — end-to-end harness в чистом окружении
- `scripts/run_mock_parity_harness.sh` — удобная обертка для запуска

## Сценарии

Harness запускает следующие сценарии в свежем workspace и с изолированными переменными окружения:

1. `streaming_text`
2. `read_file_roundtrip`
3. `grep_chunk_assembly`
4. `write_file_allowed`
5. `write_file_denied`
6. `multi_tool_turn_roundtrip`
7. `bash_stdout_roundtrip`
8. `bash_permission_prompt_approved`
9. `bash_permission_prompt_denied`
10. `plugin_tool_roundtrip`

## Запуск

```bash
cd rust/
./scripts/run_mock_parity_harness.sh
```

Поведенческий checklist / parity diff:

```bash
cd rust/
python3 scripts/run_mock_parity_diff.py
```

Связки между сценариями и `PARITY` хранятся в `mock_parity_scenarios.json`.

## Ручной запуск mock-сервера

```bash
cd rust/
cargo run -p mock-anthropic-service -- --bind 127.0.0.1:0
```

Сервер печатает `MOCK_ANTHROPIC_BASE_URL=...`; укажите этот URL в `ANTHROPIC_BASE_URL` и задайте любой непустой `ANTHROPIC_API_KEY`.
