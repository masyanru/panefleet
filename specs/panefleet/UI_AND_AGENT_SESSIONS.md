# PaneFleet: UI и восстановление сессий агентов

Статус: живая спецификация прототипа

Последнее обновление: 2026-07-27

Этот файл — единая памятка по интерфейсу PaneFleet и ожидаемому поведению
workspace, вкладок и CLI-агентов. Если реализация расходится с этим документом,
сначала уточняем продуктовую логику здесь, затем меняем код.

## 1. Основная модель интерфейса

PaneFleet использует терминальные и редакторские возможности Warp, но
организует работу вокруг проектов и нескольких параллельных CLI-агентов.

Иерархия:

```text
Application Window
├── Project Sidebar
│   ├── Workspace Header
│   └── Workspace Rows
│       └── Worktree Environment Rows
├── Workspace Surface
│   ├── Horizontal Tab Strip
│   ├── Agent Launcher Bar
│   └── Active Pane
└── Context Sidebar
    ├── Files
    ├── Changes
    └── Review
```

Термины:

- **Project Sidebar** — постоянная левая колонка со списком workspace.
- **Workspace Row** — верхнеуровневая строка одного проекта/репозитория.
- **Workspace** — проект, объединяющий исходный working tree и несколько
  worktree environment для параллельных фич.
- **Worktree Environment** — конкретный working directory и Git-ветка внутри
  workspace; environment владеет своим набором вкладок и Agent Session.
- **Worktree Environment Row** — вложенная строка `main`, `feature-a`,
  `feature-b` под одним Workspace Row.
- **Horizontal Tab Strip** — верхняя полоса вкладок активного workspace.
- **Agent Launcher Bar** — компактная строка запуска Terminal, Codex, Claude,
  OpenCode и будущих агентов.
- **Active Pane** — содержимое выбранной горизонтальной вкладки.
- **Context Sidebar** — правая панель Files / Changes / Review.
- **Agent Definition** — сохранённая конфигурация запуска и восстановления CLI.
- **Agent Session** — конкретная продолжаемая сессия агента внутри вкладки.

## 2. Project Sidebar

### 2.1 Workspace Row

Строки workspace и вложенных environment должны использовать визуальный язык
оригинального `Vertical Tab Row` в Warp, а не выглядеть как независимые
карточки.

Состав группы:

```text
[repository icon]  Project name                      [activity] [close]
                   source repository path
    [git branch]   main                               [activity] [close]
    [git branch]   feature-a                          [activity] [close]
    [git branch]   feature-b                          [activity] [close]
```

Требования:

- один исходный репозиторий показывается в Project Sidebar ровно один раз;
- название проекта показывается в верхнеуровневой строке всегда;
- иконка определяется локально: GitHub для remote на `github.com`, Git для
  остальных Git-репозиториев и мини-иконка PaneFleet для обычной папки;
- исходный working tree и managed/external worktree показываются вложенными
  environment rows; обычная не-Git папка остаётся одним невложенным workspace;
- выбор environment переключает весь Horizontal Tab Strip и Context Sidebar на
  его working directory;
- Workspace Row агрегирует активность всех вложенных environment;
- environment row показывает собственную ветку и собственную активность;
- Workspace Row разворачивается и сворачивается, не меняя порядок проектов;
- показ полного пути и текущей Git-ветки независимо настраивается в
  `Settings → Workspace`;
- при выключенном пути полный путь остаётся доступен в tooltip и контекстном
  меню;
- порядок строк стабилен и не меняется при выборе workspace;
- активная строка использует нативное selected-состояние Warp;
- hover использует нативное hover-состояние Warp;
- крестик закрытия появляется справа при hover или keyboard focus;
- закрытие workspace не удаляет проект с диска;
- если внутри есть работающий агент, перед крестиком показывается индикатор
  активности.

Контекстное меню environment:

- `Open Environment` переключает tabs и Context Sidebar на environment;
- `Reveal in Finder` открывает его локальную папку;
- `Close Environment` закрывает UI и процессы, но не удаляет файлы;
- для managed worktree доступны отдельные `Remove Worktree…` и
  `Remove Worktree and Delete Branch…`;
- внешняя прилинкованная папка удаляется только из PaneFleet: физическое
  удаление доступно лишь для managed worktree.

### 2.2 Индикатор работающих агентов

Для активной работы используется маленькая анимация из трёх точек:

```text
● · ·  →  · ● ·  →  · · ●
```

Поведение:

- анимация видна, если хотя бы одна сессия workspace выполняет реальный
  пользовательский turn: одного запущенного CLI в состоянии ожидания
  недостаточно;
- несколько одновременно работающих агентов не создают несколько анимаций:
  показывается один агрегированный индикатор;
- если активных агентов больше одного, рядом с точками показывается количество,
  а tooltip сообщает количество и имена;
- `Blocked` показывается неподвижной янтарной точкой;
- `Failed` показывается неподвижной красной точкой до просмотра пользователем;
- `Success` и отсутствие сессии не занимают место в строке;
- индикатор не должен менять ширину строки при смене кадров;
- индикатор можно полностью выключить в `Settings → Workspace`;
- при системном `Reduce Motion` вместо анимации используется неподвижная
  accent-точка;
- tooltip: `2 agents working`, `Claude is waiting for input` и аналогичные
  короткие сообщения.

Источник истины — `CLIAgentSessionsModel`, а не состояние терминального процесса
или текст заголовка вкладки. Сопоставление выполняется через
`terminal_view_id -> PaneFleet workspace`. Для `InProgress` дополнительно
требуется непустой `session_context.query`, полученный из протокольного события
агента.

## 3. Workspace Surface

### 3.1 Horizontal Tab Strip

- Каждый Worktree Environment имеет собственный набор горизонтальных вкладок.
- Смена environment целиком переключает набор вкладок.
- Вкладки другого environment не должны оставаться видимыми.
- Возврат в environment восстанавливает порядок вкладок, активную вкладку,
  pane groups и содержимое pane.
- Вкладка агента хранит не только заголовок, но и его тип, рабочую директорию,
  Agent Definition и identity сессии.
- Перед заголовком показывается маленькая иконка типа содержимого: Terminal для
  обычного терминала и иконка Agent Definition для Codex, Claude и других
  агентов.

### 3.2 Agent Launcher Bar

Панель находится непосредственно под горизонтальными вкладками.

Состав по умолчанию:

```text
[settings]  [terminal icon] Terminal  [Codex icon] Codex
            [Claude icon] Claude      [OpenCode icon] OpenCode
```

Требования:

- лёгкий визуальный вес, без больших карточек;
- маленькая иконка слева от каждого label;
- клик запускает новую вкладку в текущем workspace;
- рабочая директория новой вкладки равна корню текущего workspace;
- при восстановлении активного workspace вкладки реконструируются с
  `initial_directory = workspace root`; нативная стартовая вкладка Warp из
  каталога процесса не должна переиспользоваться как вкладка другого проекта;
- порядок и видимость launchers берутся из Agent Definitions;
- settings открывает экран конфигурации агентов;
- Terminal остаётся встроенным системным launcher и не удаляется.

### 3.3 Fleet Overview

Глобальный обзор CLI-агентов открывается кнопкой `Fleet Overview` с иконкой
группы в правой части верхней панели приложения.

Термины:

- **Fleet Overview button** — кнопка открытия обзора; accent-точка означает
  выполняющуюся работу, янтарная — сессию, требующую внимания;
- **Fleet Overview popover** — компактная плавающая панель под кнопкой;
- **Fleet Dashboard entry** — постоянная строка над workspace в Project
  Sidebar;
- **Fleet Dashboard** — полноразмерная центральная поверхность наподобие
  Settings для продолжительной работы с обзором;
- **Fleet session row** — строка конкретной CLI-сессии.

Каждая строка показывает:

```text
[agent icon] Agent · workspace
task                                      ● status  elapsed
```

Требования первой версии:

- список агрегируется из активного и parked workspace, а не только из
  отображаемых вкладок;
- popover использует непрозрачный `surface_2`; dashboard полностью заменяет
  центральную поверхность и использует `surface_1`/`surface_2`, поэтому
  терминальный текст не просвечивает;
- `InProgress` без непустого query называется `Ready`, а не `Working`;
- порядок: Working, Blocked, Failed, Ready, Done; далее workspace и агент;
- текст задачи однострочный и ограничен 96 символами;
- время считается от начала реального пользовательского turn;
- клик по строке переключает workspace, активирует правильную горизонтальную
  вкладку и фокусирует terminal view;
- панель закрывается после навигации;
- источник истины — `CLIAgentSessionsModel`.

Dashboard дополнительно показывает:

- количество работающих сессий;
- количество `Blocked` + `Failed`, требующих внимания;
- количество завершённых сессий;
- количество открытых workspace;
- responsive-сетку карточек всех верхнеуровневых workspace, включая workspace
  без обнаруженных CLI-сессий;
- внутри карточки workspace — отдельные секции worktree environment с веткой,
  локальным путём, числом сессий и собственным статусом;
- заголовок workspace агрегирует число и статус сессий всех environment;
- вложенные кликабельные карточки обнаруженных агентов;
- секцию `Recent activity` из локальной временной ленты структурированных
  lifecycle-событий;
- анимированные точки в карточке `Working`, заголовках workspace и строках
  работающих сессий.

При открытом dashboard:

- Project Sidebar и настоящая горизонтальная строка вкладок остаются на месте;
- активная терминальная вкладка визуально не выбрана;
- Files/Changes/Review inspector временно не рендерится, но его состояние не
  изменяется и восстанавливается после выхода;
- клик по workspace закрывает dashboard и открывает последнюю активную вкладку
  проекта;
- клик по агенту закрывает dashboard и фокусирует точную CLI-сессию;
- клик по обычной горизонтальной вкладке закрывает dashboard.

Dashboard не показывает выдуманные progress percentages. Временная лента
получает из `CLIAgentSessionsModel` проверяемые lifecycle-события: начало
реального turn, ожидание ввода, успешное завершение и ошибку. Для Claude
CLI-specific adapter дополнительно принимает структурированные `PostToolUse`
события и показывает tool calls, вызов skill и запуск подагента. События
сохраняются локально в `panefleet-fleet-events.json` (не более 500 последних
событий).

Event store намеренно не сохраняет prompt, response, tool input или содержимое
сессии. В записи находятся только тип события, CLI-агент, путь workspace,
время и, если провайдер его прислал, компактный идентификатор инструмента.
Process-local `terminal_view_id` используется для точной навигации лишь до
перезапуска приложения и не сериализуется; загруженное историческое событие
ведёт в соответствующий workspace.

Текущая версия Claude hook протокола передаёт для `PostToolUse` только
`tool_name`. Поэтому `Skill` и `Agent`/`Task` надёжно классифицируются как skill
и подагент, но их конкретные имена не угадываются. Dashboard показывает
`used a skill` и `spawned a subagent`; обычные и MCP-инструменты показываются по
имени. Поддержка конкретных имён требует будущего расширения структурированного
протокола, а не анализа отрисованного текста терминала.

Анимация обновляется только пока открыт Fleet UI и есть хотя бы один реальный
работающий turn.

## 4. Context Sidebar

Правая колонка не заменяет Code Review, а объединяет три режима:

- **Files** — дерево файлов выбранного workspace;
- **Changes** — существующее представление git changes;
- **Review** — существующий Code Review.

Требования:

- выбранный режим запоминается;
- при смене workspace дерево файлов и git-контекст меняются вместе с ним;
- панель можно закрывать и повторно открывать;
- Files является стартовым режимом для нового workspace;
- существующие реализации Warp для дерева и review переиспользуются.
- над деревом Files находится компактная панель действий: New File,
  New Folder, Refresh и Collapse All;
- операции используют текущее выбранное дерево, а контекстное меню отдельных
  файлов остаётся нативным меню Warp.

## 5. Settings → Agents

Экран открывается через settings в Agent Launcher Bar или общие Settings.

Компоновка:

```text
Settings navigation | Agent definitions | Selected agent editor
```

### 5.1 Список агентов

- строка поиска `Filter agents…`;
- действие `Add agent`;
- bundled definitions: Claude, Codex, OpenCode и другие поддерживаемые CLI;
- пользовательские определения;
- label и маленькая иконка;
- bundled definition можно изменить и вернуть через `Restore default`;
- пользовательское определение можно удалить.

### 5.2 Редактор Agent Definition

Поля первой версии:

- **Label** — отображаемое имя;
- **Icon** — встроенная или пользовательская маленькая иконка;
- **Launch command** — executable и аргументы запуска;
- **Prompt-only args** — аргументы, добавляемые только при наличии первого
  prompt;
- **Prompt transport** — `argv` или `stdin`;
- **Resume strategy** — встроенный adapter или настраиваемый шаблон;
- **Resume args/template** — способ передать сохранённый session ID;
- **Working directory** — по умолчанию корень workspace;
- **Environment** — только ссылки на имена переменных, без хранения секретов;
- **Permission mode** — нормальный режим или явно выбранный autonomous/bypass
  режим;
- **Enabled in launcher** и порядок в Agent Launcher Bar.

Команда должна храниться структурированно как executable + argv, где это
возможно. Секреты, токены и пароли нельзя сохранять в PaneFleet JSON.

Опасные флаги вроде `--dangerously-skip-permissions` и
`--dangerously-bypass-approvals-and-sandbox` должны быть видны пользователю в
редакторе и не добавляться скрыто.

Текущий реализованный редактор включает Label, Resume adapter, Executable,
Launch arguments, Prompt-only arguments, Prompt transport, Launcher order,
Enabled in launcher, Add agent, Delete, Restore default и Save. Пользовательская
иконка, Environment, Permission mode и произвольный resume template остаются
следующим расширением; built-in resume поведение пока выбирается через adapter.

## 5.3 Settings → Workspace

Отдельная страница управляет плотностью Workspace Row:

- **Show workspace path** — полный корень проекта под названием;
- **Show Git branch** — текущая ветка или короткий hash detached HEAD;
- **Show agent activity** — анимированные и статические индикаторы агентов;
- **Confirm closing the final tab** — запрашивать подтверждение перед закрытием
  последней вкладки workspace.

Из навигации Settings в режиме PaneFleet скрыты продуктовые разделы Warp,
которые не относятся к локальному workbench: Warp Agent, Profiles, Knowledge,
Billing and usage, Cloud platform, Teams, Warpify, Referrals и Shared blocks.
Account пока сохраняется, а судьба Warp Drive будет определена отдельно.

## 5.4 Settings → Notifications

Первая версия намеренно минимальна:

- один переключатель звука завершения агентского хода;
- три ненавязчивых системных звука macOS: `Glass`, `Pop` и `Tink`;
- выбор звука и кнопка Preview;
- настройка хранится локально и применяется к новым событиям сразу.

Звук воспроизводится только после перехода реального пользовательского turn из
`InProgress` в `Success` или `Failed`. Сам запуск CLI, восстановление сохранённой
сессии, idle prompt и переход в `Blocked` уведомлением не считаются.

Источник истины — структурированные события `CLIAgentSessionsModel`. PaneFleet
запоминает `terminal_view_id` только после `InProgress` с непустым query и
удаляет его после первого завершающего события, поэтому один turn даёт не более
одного звука. Унаследованный стандартный звук desktop notification в режиме
PaneFleet выключен, чтобы не было двойного сигнала.

## 6. Точное восстановление Agent Session

### 6.1 Критерий готовности

Восстановление считается успешным, только если после перезапуска PaneFleet:

1. открывается тот же workspace;
2. возвращается та же горизонтальная вкладка;
3. запускается тот же тип агента;
4. используется сохранённый session/conversation ID;
5. виден предыдущий transcript;
6. следующий prompt продолжает ту же беседу.

Просто открыть терминал с прежним заголовком — **не восстановление**.
Если resume невозможен, PaneFleet показывает явную ошибку и действие
`Start new session`; тихо создавать новую сессию запрещено.

### 6.2 Persisted Agent Tab

Для агентской вкладки сохраняются:

```text
workspace_id
tab_id
tab_order
tab_title
agent_definition_id
agent_kind
working_directory
terminal/pane snapshot identity
provider_session_id
warp_conversation_id (если существует)
resume_metadata
last_known_status
last_seen_at
```

`provider_session_id` берётся из `CLIAgentSessionContext.session_id`.
Состояние обновляется на `Started`, `SessionUpdated`, `StatusChanged` и перед
штатным закрытием приложения.

### 6.3 Resume adapters

Каждый агент получает adapter с единым контрактом:

```text
launch(definition, cwd, optional_prompt)
resume(definition, cwd, provider_session_id, optional_prompt)
detect_session_id(events/output)
validate_resume_state(provider_session_id, cwd)
```

Минимальные встроенные adapters:

- **Claude**: новая сессия получает заранее созданный UUID через
  `--session-id`; восстановление использует `claude --resume <uuid>`.
- **Codex**: ID получается после старта из событий; восстановление использует
  `codex resume <session_id>`. При запуске из процесса, который сам работает
  внутри Codex, adapter удаляет у дочернего процесса родительские
  `CODEX_THREAD_ID` и `CODEX_CI`, чтобы новая вкладка получила независимую
  conversation identity.
- **OpenCode**: provider session ID является непрозрачной строкой вида
  `ses_…`; восстановление использует `opencode -s <provider_session_id>`.
- **Terminal**: восстанавливается обычным snapshot-механизмом Warp без Agent
  Definition.

Claude activity adapter переиспользует rich integration protocol Warp:

- `PostToolUse` приходит как `ToolComplete` с безопасным `tool_name`;
- `Skill` классифицируется как skill activity;
- `Agent` и прежнее имя `Task` классифицируются как subagent activity;
- остальные имена, включая `mcp__…`, классифицируются как tool activity;
- tool input, output, prompt и transcript не копируются в Fleet events.

Существующие `ClaudeHarness` и `CodexHarness` в Warp уже содержат рабочую
логику resume и должны быть переиспользованы, а не продублированы строковыми
shell-командами.

### 6.4 Последовательность восстановления

1. Загрузить список workspace и их вкладок.
2. Создать визуальный tab strip в состоянии `Restoring`.
3. Для Terminal применить нативный snapshot Warp.
4. Для Agent проверить Agent Definition и наличие session ID.
5. Проверить локальный transcript/index, если этого требует adapter.
6. Запустить `resume`, привязать новый `terminal_view_id` к сохранённому tab ID.
7. Дождаться подтверждения session ID от CLI/plugin events.
8. Только после совпадения identity снять состояние `Restoring`.
9. При ошибке оставить вкладку на месте с объяснением и действиями
   `Retry`, `Open transcript`, `Start new session`, `Close tab`.

Командный detector должен видеть агента и за системным wrapper, например
`env -u CODEX_THREAD_ID -u CODEX_CI codex resume <id>`. После старта ожидаемого
процесса прогресс-баннер скрывается, но внутреннее состояние `Confirming`
остаётся до plugin event. Совпавший session ID завершает проверку; другой ID
переводит вкладку в `Failed`. Для provider hook без session ID допустим
best-effort confirm по rich-событию, полученному после запуска resume-команды.

## 7. Состояния вкладки агента

```text
Restoring  → InProgress → Blocked → InProgress → Success
     │           │                         │
     └───────────┴─────────────────────────┴──→ Failed
```

- `Restoring` — session identity загружена, resume ещё не подтверждён;
- `InProgress` — агент выполняет работу;
- `Blocked` — требуется ввод, подтверждение или разрешение пользователя;
- `Success` — последний turn завершён, сессию можно продолжить;
- `Failed` — запуск или resume завершился ошибкой;
- закрытие вкладки отдельно от завершения сессии: пользователь может закрыть
  представление, не удаляя transcript/history.

## 8. Хранение данных

Текущее состояние прототипа:

```text
~/Library/Application Support/dev.panefleet.PaneFleet/
├── panefleet-workspaces.json
├── panefleet-agent-definitions.json
├── panefleet-workspace-preferences.json
└── panefleet-notification-preferences.json
```

Перед реализацией точного resume формат нужно версионировать:

```json
{
  "version": 2,
  "active_workspace_id": "...",
  "workspaces": []
}
```

Требования:

- атомарная запись через временный файл и rename;
- неизвестные новые поля игнорируются старой версией;
- миграция с текущего формата без потери workspace;
- никакой конфиденциальной информации и токенов;
- session IDs допустимы, но не должны попадать в telemetry или обычные логи.

## 9. Порядок реализации

### P0 — корректность сессий

- [x] Версионировать persisted schema.
- [x] Сохранять agent kind и provider session ID на уровне вкладки.
- [x] Реализовать общий resume adapter.
- [x] Подключить Claude resume.
- [x] Подключить Codex resume.
- [x] Подключить OpenCode resume через `opencode -s <session_id>`.
- [x] Показывать явное состояние ошибки вместо пустого терминала.
- [ ] Добавить тест: несколько workspace и несколько агентов переживают полный
      перезапуск приложения.

### P1 — конфигурация CLI

- [x] Добавить модель Agent Definition.
- [x] Сделать Settings → Agents.
- [x] Подключить launcher к сохранённым определениям.
- [x] Добавить bundled defaults и Restore default.
- [x] Добавить пользовательские определения и валидацию команд.
- [ ] Добавить пользовательские иконки, Environment, Permission mode и
      произвольные resume templates.

### P2 — workspace activity UI

- [x] Сделать путь и Git-ветку опциональными элементами Workspace Row.
- [x] Связать терминальные view с workspace.
- [x] Агрегировать статусы через `CLIAgentSessionsModel`.
- [x] Добавить трёхточечную анимацию.
- [ ] Добавить Reduce Motion fallback.
- [x] Добавить Blocked/Failed состояния и tooltips.
- [x] Подключить структурированные Claude tool/skill/subagent events к Fleet.
- [ ] Расширить Claude hook protocol конкретными именами skill и подагента.
- [ ] Добавить live-состояния Claude tool/skill/subagent и сворачивание
      низкоуровневых tool calls в `Recent activity`.

### P3 — environment isolation через Git worktrees

Цель — параллельно работать с одной исходной репой на нескольких Git-ветках,
не разделяя один working tree между агентами. Исходный репозиторий остаётся
одним верхнеуровневым PaneFleet workspace, а каждая ветка представлена
вложенным worktree environment.

- [x] Добавить отдельные действия `Existing folder` и `Isolated worktree` в
      заголовок Project Sidebar.
- [x] Для isolated environment выбирать базовую ветку и создавать новую ветку.
- [x] Создавать отдельный Git worktree и использовать его корень как `cwd` для
      всех Terminal и Agent Session этого environment.
- [x] Направлять Files / Changes / Review и Git metadata на конкретный worktree,
      а не на исходную папку репозитория.
- [x] Сохранять связь `workspace → worktree path → branch → environment`
      в версионированном локальном состоянии.
- [x] Показывать исходный working tree и все связанные worktree как вложенные
      environment rows под одной строкой проекта в Project Sidebar.
- [x] Показывать во Fleet Dashboard одну карточку проекта с отдельными
      environment-секциями и агрегированным статусом.
- [x] Мигрировать сохранённые top-level worktree-записи обратно под исходный
      репозиторий без потери вкладок и Agent Session.
- [ ] Поддержать подключение уже существующего внешнего worktree без его
      переноса под управление PaneFleet.
- [x] Проверять конфликт ветки, занятый путь и незакоммиченные изменения до
      создания managed worktree.
- [x] При закрытии environment не удалять worktree или Git-ветку автоматически.
- [x] Добавить отдельное подтверждаемое действие очистки с проверкой dirty state,
      которое никогда не удаляет Git-ветку неявно.
- [x] Блокировать cleanup при modified/untracked файлах, закрывать процессы
      environment перед удалением и удалять worktree через `git worktree
      remove`, а не через рекурсивное удаление папки.
- [x] Оставлять ветку по умолчанию; явное удаление ветки выполнять только через
      безопасный `git branch -d`, сохраняя unmerged-ветку при отказе Git.
- [x] Гарантировать, что агенты двух isolated environment не получают одинаковый
      working directory.
- [ ] После изоляции добавить CLI-specific activity adapters для Codex и
      OpenCode.

### P4 — notifications

- [x] Добавить локальные настройки звука завершения agent turn.
- [x] Добавить три системных macOS-звука и Preview.
- [x] Не уведомлять о старте CLI, resume, idle и Blocked.
- [x] Исключить двойной системный звук у унаследованных desktop notifications.

## 10. Acceptance checklist

- Выбор workspace не меняет порядок строк слева.
- Путь, Git-ветка и agent activity в Project Sidebar подчиняются настройкам
  Workspace.
- У каждого workspace свой tab strip.
- Работающий агент заметен в строке workspace, даже если выбран другой проект.
- Launcher запускает CLI по пользовательской конфигурации.
- Files / Changes / Review относятся к активному workspace.
- После полного restart все восстанавливаемые Agent Session продолжают прежние
  беседы.
- Два environment одного workspace могут одновременно работать на разных
  ветках через разные Git worktrees и не изменяют файлы друг друга.
- Files / Changes / Review и все новые Agent Session используют worktree
  выбранного environment.
- Закрытие environment не удаляет его worktree или ветку без отдельного
  подтверждённого действия.
- Невозможность resume никогда не маскируется новой пустой сессией.
- UI использует существующие темы, кнопки, иконки и состояния Warp.
