# PaneFleet: UI и восстановление сессий агентов

Статус: живая спецификация прототипа

Последнее обновление: 2026-07-25

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
- **Workspace Row** — строка одного проекта в левой колонке.
- **Workspace** — проект и принадлежащий ему набор вкладок.
- **Horizontal Tab Strip** — верхняя полоса вкладок активного workspace.
- **Agent Launcher Bar** — компактная строка запуска Terminal, Codex, Claude,
  OpenCode и будущих агентов.
- **Active Pane** — содержимое выбранной горизонтальной вкладки.
- **Context Sidebar** — правая панель Files / Changes / Review.
- **Agent Definition** — сохранённая конфигурация запуска и восстановления CLI.
- **Agent Session** — конкретная продолжаемая сессия агента внутри вкладки.

## 2. Project Sidebar

### 2.1 Workspace Row

Строки workspace должны выглядеть и вести себя как оригинальные
`Vertical Tab Row` в Warp, а не как отдельный новый компонент.

Состав строки:

```text
[project/terminal icon]  Project name       [activity] [close]
```

Требования:

- показывать только название проекта;
- путь к проекту в обычном состоянии не показывать;
- полный путь оставить в tooltip и при необходимости в контекстном меню;
- порядок строк стабилен и не меняется при выборе workspace;
- активная строка использует нативное selected-состояние Warp;
- hover использует нативное hover-состояние Warp;
- крестик закрытия появляется справа при hover или keyboard focus;
- закрытие workspace не удаляет проект с диска;
- если внутри есть работающий агент, перед крестиком показывается индикатор
  активности.

### 2.2 Индикатор работающих агентов

Для активной работы используется маленькая анимация из трёх точек:

```text
● · ·  →  · ● ·  →  · · ●
```

Поведение:

- анимация видна, если хотя бы одна сессия workspace имеет статус
  `InProgress`;
- несколько одновременно работающих агентов не создают несколько анимаций:
  показывается один агрегированный индикатор;
- если активных агентов больше одного, tooltip сообщает количество и имена;
- `Blocked` показывается неподвижной янтарной точкой;
- `Failed` показывается неподвижной красной точкой до просмотра пользователем;
- `Success` и отсутствие сессии не занимают место в строке;
- индикатор не должен менять ширину строки при смене кадров;
- при системном `Reduce Motion` вместо анимации используется неподвижная
  accent-точка;
- tooltip: `2 agents working`, `Claude is waiting for input` и аналогичные
  короткие сообщения.

Источник истины — `CLIAgentSessionsModel`, а не состояние терминального процесса
или текст заголовка вкладки. Сопоставление выполняется через
`terminal_view_id -> PaneFleet workspace`.

## 3. Workspace Surface

### 3.1 Horizontal Tab Strip

- Каждый workspace имеет собственный набор горизонтальных вкладок.
- Смена workspace целиком переключает набор вкладок.
- Вкладки другого workspace не должны оставаться видимыми.
- Возврат в workspace восстанавливает порядок вкладок, активную вкладку,
  pane groups и содержимое pane.
- Вкладка агента хранит не только заголовок, но и его тип, рабочую директорию,
  Agent Definition и identity сессии.

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
- порядок и видимость launchers берутся из Agent Definitions;
- settings открывает экран конфигурации агентов;
- Terminal остаётся встроенным системным launcher и не удаляется.

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
  `codex resume <session_id>`.
- **OpenCode**: adapter реализуется после проверки стабильного CLI-контракта
  текущей версии; до этого UI обязан честно показывать `Resume unsupported`.
- **Terminal**: восстанавливается обычным snapshot-механизмом Warp без Agent
  Definition.

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
└── panefleet-workspaces.json
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
- [x] Показывать явное состояние ошибки вместо пустого терминала.
- [ ] Добавить тест: несколько workspace и несколько агентов переживают полный
      перезапуск приложения.

### P1 — конфигурация CLI

- [ ] Добавить модель Agent Definition.
- [ ] Сделать Settings → Agents.
- [ ] Подключить launcher к сохранённым определениям.
- [ ] Добавить bundled defaults и Restore default.
- [ ] Добавить пользовательские определения и валидацию команд.

### P2 — workspace activity UI

- [ ] Убрать путь из обычного состояния Workspace Row.
- [ ] Связать терминальные view с workspace.
- [ ] Агрегировать статусы через `CLIAgentSessionsModel`.
- [ ] Добавить трёхточечную анимацию и Reduce Motion fallback.
- [ ] Добавить Blocked/Failed состояния и tooltips.

## 10. Acceptance checklist

- Выбор workspace не меняет порядок строк слева.
- В Project Sidebar нет постоянно отображаемых путей.
- У каждого workspace свой tab strip.
- Работающий агент заметен в строке workspace, даже если выбран другой проект.
- Launcher запускает CLI по пользовательской конфигурации.
- Files / Changes / Review относятся к активному workspace.
- После полного restart все восстанавливаемые Agent Session продолжают прежние
  беседы.
- Невозможность resume никогда не маскируется новой пустой сессией.
- UI использует существующие темы, кнопки, иконки и состояния Warp.
