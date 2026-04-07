# Запуск Claw Code в Docker

Самодостаточная инструкция для работы **только через Docker**. Общий runbook по конфигурации и аутентификации: [SETUP_AND_OPERATIONS.md](./SETUP_AND_OPERATIONS.md).

## Требования

- Установленный Docker (Desktop или Engine).
- Клон репозитория; команды ниже выполняются из **корня репозитория** (рядом с `Dockerfile`).

Рабочая директория в контейнере: **`/workspace`** — туда монтируется ваш проект.

## Сборка образа

```bash
cd "$(git rev-parse --show-toplevel)"
docker build -t claw-code .
```

В образе уже собраны бинарники **`claw`** (CLI) и **`claw-web`** (веб-UI), а также `git`, `python3`, `ripgrep` и инструментарий Rust.

## Переменные окружения и `.env`

В корне репозитория есть шаблон **`.env.example`**. Скопируйте и заполните нужные переменные:

```bash
cp .env.example .env
```

Файл **`.env` в корне смонтированного репозитория** подхватывается при работе из `/workspace`. Для **`claw-web`** то же верно при **`--cwd /workspace`**.

| Сценарий | Минимум |
| --- | --- |
| Прямой Anthropic API | `ANTHROPIC_API_KEY` |
| OpenAI-compatible (в т.ч. локальная модель) | `OPENAI_API_KEY`; при необходимости `OPENAI_BASE_URL`, `OPENAI_MODEL` |

Если LLM слушает **на хосте**, а контейнер в Docker Desktop (macOS/Windows), для `OPENAI_BASE_URL` часто используют `http://host.docker.internal:<порт>`. На Linux может понадобиться `docker run` с `--add-host=host.docker.internal:host-gateway`.

Не коммитьте `.env` (файл в `.gitignore`).

## CLI: интерактивный режим

```bash
cd "$(git rev-parse --show-toplevel)"

docker run --rm -it \
  -v "$PWD":/workspace \
  -w /workspace \
  claw-code
```

Ключи из `.env` подтянутся автоматически, если файл лежит в корне смонтированного репо.

Передать ключ явно (без `.env`):

```bash
docker run --rm -it \
  -e ANTHROPIC_API_KEY="ваш-ключ" \
  -v "$PWD":/workspace \
  -w /workspace \
  claw-code
```

## CLI: один запрос

```bash
docker run --rm -it \
  -v "$PWD":/workspace \
  -w /workspace \
  claw-code prompt "кратко опиши этот репозиторий"
```

## CLI: провайдер OpenAI-compatible

После настройки `.env` (`OPENAI_*`):

```bash
docker run --rm -it \
  -v "$PWD":/workspace \
  -w /workspace \
  claw-code \
  --provider openai-compatible \
  --model имя-вашей-модели
```

Идентификатор модели должен совпадать с тем, что ожидает ваш совместимый сервер.

## Веб-интерфейс (`claw-web`)

Нужны порты **8787** (UI) и **4545** (OAuth callback при входе через браузер).

Фиксированное имя **`--name claw-web`** удобно для логов и повторных запусков, но если контейнер с таким именем уже есть (прошлый сеанс не завершился или контейнер остался без удаления), Docker ответит: `The container name "/claw-web" is already in use`. Перед новым запуском **остановите и удалите** старый экземпляр:

```bash
docker rm -f claw-web
```

Ниже эта команда встроена в сценарий (ошибка «нет такого контейнера» при первом запуске можно игнорировать — stderr подавлен).

```bash
cd "$(git rev-parse --show-toplevel)"
mkdir -p "$HOME/.claw-docker"
docker rm -f claw-web 2>/dev/null || true

docker run --rm -it \
  --name claw-web \
  -p 8787:8787 \
  -p 4545:4545 \
  -v "$PWD":/workspace \
  -v "$HOME/.claw-docker:/root/.claw" \
  --entrypoint claw-web \
  claw-code \
  --cwd /workspace
```

Откройте в браузере: **http://localhost:8787**

### Локальная OpenAI-compatible модель в Web UI

Сервер (`claw-web`) читает `.env` из каталога **`--cwd`** (в примере выше это `/workspace`, то есть корень смонтированного репозитория). После правок `.env` **перезапустите контейнер** — переменные подхватываются при старте процесса.

**1. Переменные окружения**

| Переменная | Назначение |
| --- | --- |
| `OPENAI_API_KEY` | **Обязательна**, иначе в UI провайдер не считается готовым к запросам. У многих локальных совместимых серверов достаточно произвольной непустой строки (например тот же ключ, что вы уже используете для локали). |
| `OPENAI_BASE_URL` | Базовый URL OpenAI-compatible API, обычно с суффиксом **`/v1`** (как в `.env.example`: `https://api.openai.com/v1`). Для сервера на **хосте** из контейнера часто: `http://host.docker.internal:<порт>/v1` (Docker Desktop на macOS/Windows). На Linux при необходимости: `docker run ... --add-host=host.docker.internal:host-gateway`. |
| `OPENAI_MODEL` | Необязательный **дефолт**: подставляется в health-ответ и может автоматически попасть в поле **Model**, если вы не вводили своё значение. Если в поле **Model** уже есть текст, запрос уходит с ним; пустое поле и пустой `OPENAI_MODEL` дают ошибку вида «no model specified…». |

**2. Действия в интерфейсе**

В левой колонке (**Controls**):

1. **Provider** — выберите **`openai-compatible`** (не полагайтесь на `auto`, если у вас одновременно заданы другие ключи: порядок выбора в `auto` может увести на другой провайдер).
2. **Model** — введите **точный идентификатор модели**, который ожидает ваш сервер (как в его OpenAI-compatible `/v1/chat/completions`): имя в Ollama, имя в vLLM, `model` в LM Studio и т.д. Регистр сохраняется так, как вы ввели в поле.

**3. Проверка**

В блоке **Auth** статус должен показывать наличие `OPENAI_API_KEY`; кнопка отправки сообщения активна, когда выбранный провайдер «готов» и поле **Model** не пустое. Текст-подсказка в UI напоминает про `OPENAI_BASE_URL`, если сервер не на стандартном `https://api.openai.com/v1`.

**4. Примеры**

Только через `.env` в корне репозитория (файл на хосте, том `-v "$PWD":/workspace`):

```env
OPENAI_API_KEY=local-dev-key
OPENAI_BASE_URL=http://host.docker.internal:11434/v1
OPENAI_MODEL=qwen2.5
```

Те же значения явно в `docker run` (удобно для одноразового теста):

```bash
docker run --rm -it \
  -p 8787:8787 \
  -p 4545:4545 \
  -e OPENAI_API_KEY="local-dev-key" \
  -e OPENAI_BASE_URL="http://host.docker.internal:11434/v1" \
  -e OPENAI_MODEL="qwen2.5" \
  -v "$PWD":/workspace \
  -v "$HOME/.claw-docker:/root/.claw" \
  --entrypoint claw-web \
  claw-code \
  --cwd /workspace
```

Тот же запуск без правки `.env` на диске: передайте нужные `-e` к той же команде, что в начале раздела «Веб-интерфейс» (образ, том `-v "$PWD":/workspace`, при желании `-v "$HOME/.claw-docker:/root/.claw"`).

**OAuth в браузере (Claude):** не передавайте в контейнер `ANTHROPIC_API_KEY` и `ANTHROPIC_AUTH_TOKEN`, иначе они перекроют сохранённые OAuth-токены. Callback: `http://localhost:4545/callback` — без `-p 4545:4545` редирект в контейнер не дойдёт.

Ограничение runtime (см. [SETUP_AND_OPERATIONS.md](./SETUP_AND_OPERATIONS.md)): для прямых запросов к `https://api.anthropic.com` по-прежнему нужен **`ANTHROPIC_API_KEY`**; OAuth-only inference для Anthropic в этом репозитории не реализован. Сохранённые credentials полезны при upstream с bearer через `ANTHROPIC_BASE_URL`.

## Только `claw login` (OAuth) в Docker

```bash
mkdir -p "$HOME/.claw-docker"

docker run --rm -it \
  -p 4545:4545 \
  -v "$PWD":/workspace \
  -v "$HOME/.claw-docker:/root/.claw" \
  -w /workspace \
  claw-code login
```

## Сводка

| Задача | Entrypoint | Тома | Порты |
| --- | --- | --- | --- |
| CLI | по умолчанию `claw` | `-v "$PWD":/workspace`, `-w /workspace` | — |
| Web UI | `--entrypoint claw-web` … `--cwd /workspace` | то же + `$HOME/.claw-docker:/root/.claw` | `8787`, `4545` |

Дублирующие фрагменты и матрица запуска также есть в [SETUP_AND_OPERATIONS.md](./SETUP_AND_OPERATIONS.md) (раздел «Установка и запуск через Docker»).
