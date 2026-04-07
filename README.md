# Переписывание проекта Claw Code

<p align="center">
  <strong>⭐ Самый быстрый репозиторий в истории, достигший 50K звезд: эта отметка была взята всего за 2 часа после публикации ⭐</strong>
</p>

<p align="center">
  <a href="https://star-history.com/#ultraworkers/claw-code&Date">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ultraworkers/claw-code&type=Date&theme=dark" />
      <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ultraworkers/claw-code&type=Date" />
      <img alt="График истории звезд" src="https://api.star-history.com/svg?repos=ultraworkers/claw-code&type=Date" width="600" />
    </picture>
  </a>
</p>

<p align="center">
  <img src="assets/clawd-hero.jpeg" alt="Claw" width="300" />
</p>

<p align="center">
  <strong>Репозиторий автономно поддерживается lobsters/claws, а не человеческими руками</strong>
</p>

<p align="center">
  <a href="https://github.com/Yeachan-Heo/clawhip">clawhip</a> ·
  <a href="https://github.com/code-yeongyu/oh-my-openagent">oh-my-openagent</a> ·
  <a href="https://github.com/Yeachan-Heo/oh-my-claudecode">oh-my-claudecode</a> ·
  <a href="https://github.com/Yeachan-Heo/oh-my-codex">oh-my-codex</a> ·
  <a href="https://discord.gg/6ztZB9jvWq">Discord UltraWorkers</a>
</p>

> [!IMPORTANT]
> Активный Rust workspace теперь находится в [`rust/`](./rust). Начните с [`USAGE.md`](./USAGE.md), если нужны быстрые команды, затем откройте [`docs/SETUP_AND_OPERATIONS.md`](./docs/SETUP_AND_OPERATIONS.md) для полного runbook по установке и эксплуатации. Запуск в Docker — [`docs/DOCKER.md`](./docs/DOCKER.md). Детали по crate’ам — [`rust/README.md`](./rust/README.md).

> Нужна более широкая идея, стоящая за этим репозиторием? Прочитайте [`PHILOSOPHY.md`](./PHILOSOPHY.md) и публичное объяснение Сигрид Джин: https://x.com/realsigridjin/status/2039472968624185713

> Отдельное спасибо экосистеме UltraWorkers, которая двигает этот репозиторий: [clawhip](https://github.com/Yeachan-Heo/clawhip), [oh-my-openagent](https://github.com/code-yeongyu/oh-my-openagent), [oh-my-claudecode](https://github.com/Yeachan-Heo/oh-my-claudecode), [oh-my-codex](https://github.com/Yeachan-Heo/oh-my-codex) и [Discord UltraWorkers](https://discord.gg/6ztZB9jvWq).

---

## Предыстория

Этот репозиторий поддерживается **lobsters/claws**, а не обычной командой, состоящей только из людей.

За системой стоят [Bellman / Yeachan Heo](https://github.com/Yeachan-Heo) и его коллеги, например [Yeongyu](https://github.com/code-yeongyu), но сам репозиторий развивается через автономные claw-workflow: параллельные coding-сессии, событийную оркестрацию, recovery loops и машинно-читаемое состояние lane.

На практике это означает, что этот проект не просто *о* coding-агентах — он **активно строится ими**. Возможности, тесты, телеметрия, документация и hardening рабочих процессов вносятся через claw-driven loops с использованием [clawhip](https://github.com/Yeachan-Heo/clawhip), [oh-my-openagent](https://github.com/code-yeongyu/oh-my-openagent), [oh-my-claudecode](https://github.com/Yeachan-Heo/oh-my-claudecode) и [oh-my-codex](https://github.com/Yeachan-Heo/oh-my-codex).

Этот репозиторий существует как доказательство того, что открытый coding harness можно строить **автономно, публично и на высокой скорости** — когда люди задают направление, а claws выполняют тяжелую работу.

Публичная история сборки:

https://x.com/realsigridjin/status/2039472968624185713

![Скриншот твита](assets/tweet-screenshot.png)

---

## Статус портирования

Основное дерево исходников сейчас Python-first.

- `src/` содержит активный Python-workspace для портирования
- `tests/` проверяет текущее Python-workspace
- открытый snapshot больше не является частью отслеживаемого состояния репозитория

Текущее Python-workspace пока не является полной one-to-one заменой исходной системы, но основной поверхностью реализации сейчас считается Python.

## Зачем существует это переписывание

Изначально я изучал открытый код, чтобы понять harness, связку инструментов и агентный workflow. После более глубокого погружения в юридические и этические вопросы, а также после прочтения эссе по ссылке ниже, я не захотел, чтобы сам открытый snapshot оставался основным отслеживаемым исходным деревом.

Теперь этот репозиторий сосредоточен на Python-портировании.

## Структура репозитория

```text
.
├── src/                                # Python-workspace для портирования
│   ├── __init__.py
│   ├── commands.py
│   ├── main.py
│   ├── models.py
│   ├── port_manifest.py
│   ├── query_engine.py
│   ├── task.py
│   └── tools.py
├── tests/                              # Проверка Python-части
├── assets/omx/                         # Скриншоты OmX workflow
├── 2026-03-09-is-legal-the-same-as-legitimate-ai-reimplementation-and-the-erosion-of-copyleft.md
└── README.md
```

## Обзор Python-workspace

Новая Python-ветка `src/` сейчас предоставляет:

- **`port_manifest.py`** — сводку по текущей структуре Python-workspace
- **`models.py`** — dataclass-модели для подсистем, модулей и состояния backlog
- **`commands.py`** — Python-метаданные по портированию команд
- **`tools.py`** — Python-метаданные по портированию инструментов
- **`query_engine.py`** — формирование сводки по Python-портированию из активного workspace
- **`main.py`** — CLI entrypoint для вывода manifest и summary

## Быстрый старт

Показать summary по Python-портированию:

```bash
python3 -m src.main summary
```

Вывести manifest текущего Python-workspace:

```bash
python3 -m src.main manifest
```

Показать текущие Python-модули:

```bash
python3 -m src.main subsystems --limit 16
```

Запустить проверку:

```bash
python3 -m unittest discover -s tests -v
```

Запустить parity audit по локальному игнорируемому архиву, если он присутствует:

```bash
python3 -m src.main parity-audit
```

Просмотреть зеркальные inventory команд и инструментов:

```bash
python3 -m src.main commands --limit 10
python3 -m src.main tools --limit 10
```

## Текущий parity-checkpoint

Порт теперь намного точнее отражает поверхность архивного root-entry файла, имена верхнеуровневых подсистем и inventory команд/инструментов. Однако это **еще не** полноценная runtime-эквивалентная замена исходной TypeScript-системы; в Python-дереве по-прежнему меньше исполняемых runtime-срезов, чем в архивном исходнике.

## Создано с помощью `oh-my-codex`

Реструктуризация и работа над документацией в этом репозитории выполнялись с AI-поддержкой и оркестрировались через [oh-my-codex (OmX)](https://github.com/Yeachan-Heo/oh-my-codex) Йечана Хо, поверх Codex.

- **режим `$team`:** использовался для координированного параллельного ревью и архитектурной обратной связи
- **режим `$ralph`:** использовался для устойчивого исполнения, проверки и дисциплины доведения до конца
- **workflow на базе Codex:** использовался для превращения основного дерева `src/` в Python-first workspace для портирования

### Скриншоты OmX workflow

![Скриншот OmX workflow 1](assets/omx/omx-readme-review-1.png)

*Вид orchestration через Ralph/team во время ревью README и контекста эссе в терминальных панелях.*

![Скриншот OmX workflow 2](assets/omx/omx-readme-review-2.png)

*Двухпанельный поток ревью и верификации во время финального прохода по формулировкам README.*

## Сообщество

<p align="center">
  <a href="https://discord.gg/6ztZB9jvWq"><img src="https://img.shields.io/badge/UltraWorkers-Discord-5865F2?logo=discord&style=for-the-badge" alt="Discord UltraWorkers" /></a>
</p>

Присоединяйтесь к [**Discord UltraWorkers**](https://discord.gg/6ztZB9jvWq) — это сообщество вокруг clawhip, oh-my-openagent, oh-my-claudecode, oh-my-codex и claw-code. Там обсуждают LLM, проектирование harness-систем, агентные workflow и автономную разработку ПО.

[![Discord](https://img.shields.io/badge/Join%20Discord-UltraWorkers-5865F2?logo=discord&style=for-the-badge)](https://discord.gg/6ztZB9jvWq)

## История звезд

Смотрите график в верхней части этого README.

## Дисклеймер по владению / аффилиации

- Этот репозиторий **не** заявляет права собственности на исходные материалы Claude Code.
- Этот репозиторий **не аффилирован с Anthropic, не одобрен ею и не поддерживается ею**.
