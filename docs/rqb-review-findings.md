# Итоги ревью rqb: проблемы и план исправлений

Дата: 2026-05-12

Документ фиксирует результаты первичного анализа кодовой базы `rqb` и независимой
перепроверки через `педант`. Цель - собрать все найденные риски в порядке
важности, чтобы дальше чинить не по вдохновению, а по ущербу. Да, звучит почти
как взрослая инженерия, неприятно, но полезно.

## Контекст

`rqb` - Rust query builder для Postgres поверх `sqlx`. Это не ORM: серверный
Rust-код владеет формой SQL-запроса, а клиентский JSON ограничен поисковым
адаптером (`SearchRequest`) для фильтрации, сортировки, `limit` и `offset` через
явно exposed metadata.

Основные части репозитория:

- `crates/rqb` - core AST, валидация, render, execution helpers.
- `crates/rqb-macros` - `schema!`, `Insertable`, `Changeset`.
- `crates/rqb-cli` - introspection Postgres и генерация schema-модуля.
- `samples` - executable API documentation.
- `crates/rqb/tests` и `test` - unit/integration/Docker checks.

## P1: корректность и безопасность

### 1. Raw placeholder parser не понимает SQL-контекст

Где:

- `crates/rqb/src/raw.rs`
- `crates/rqb/src/render/raw.rs`

Что нашли:

Raw-фрагменты считают каждый `?` как bind placeholder, кроме экранированного
`??`. Парсер не понимает:

- single-quoted literals: `'foo?bar'`;
- dollar-quoted bodies: `$tag$ ... ? ... $tag$`;
- line/block comments: `-- ?`, `/* ? */`;
- quoted identifiers: `"weird?col"`.

Риск:

Пользователь получает либо ложный `RawBindMismatch`, либо некорректную замену
символа `?` на `$N`. Это не SQL injection в типичном смысле, но это реальная
корректность raw escape hatch, а raw escape hatch обычно используют как раз там,
где больно.

Что делать:

Сделать минимальный SQL lexer для raw placeholder handling. Он должен пропускать
`?` внутри single quotes, dollar quotes, double-quoted identifiers, line comments
и block comments. Добавить тесты на все перечисленные случаи.

### 2. `SearchFilterWire` молча проглатывает лишние поля

Где:

- `crates/rqb/src/request/ast.rs`

Что нашли:

Верхние структуры `SearchRequest`, `SearchPredicate`, `SearchSort` используют
`#[serde(deny_unknown_fields)]`, но внутренний `SearchFilterWire` сделан как
`#[serde(untagged)]` enum без строгого запрета лишних полей на вариантах.

Примеры проблемных payload:

```json
{ "and": [], "extra": "ignored" }
```

```json
{ "and": [], "field": "status", "operator": "equals", "value": "paid" }
```

```json
{ "and": [], "or": [] }
```

Из-за `untagged` serde выбирает первый подходящий вариант и может молча
выбросить остальные ключи.

Риск:

API-контракт слабее, чем выглядит. Клиент может отправить неоднозначный или
мусорный фильтр, а сервер примет его как валидный. Для search API это плохая
предсказуемость и потенциально неприятные баги на границе сервиса.

Что делать:

Минимальный вариант: добавить строгую десериализацию для wire-вариантов и тесты
на ambiguous payloads. Более чистый вариант: перейти на tagged shape, например
`{ "kind": "and", "filters": [...] }`, но это уже breaking API.

### 3. `Error::Connection` почти мёртвый, retry policy врёт

Где:

- `crates/rqb/src/error.rs`

Что нашли:

`Error::Connection(String)` существует, и `is_retryable()` считает его
retryable. Но `From<sqlx::Error>` мапит только `RowNotFound` и `Database`.
Остальные ошибки `sqlx`, включая connection-like failures, уходят в
`Error::Sqlx`.

Проблемные классы:

- `sqlx::Error::Io`;
- `sqlx::Error::Tls`;
- `sqlx::Error::PoolClosed`;
- `sqlx::Error::PoolTimedOut`;
- `sqlx::Error::WorkerCrashed`.

Риск:

`is_retryable()` возвращает `false` для реальных transient connection failures.
Для production retry policy это уже не косметика, а неприятная поломка
поведения.

Что делать:

Либо мапить connection-like `sqlx::Error` в `Error::Connection`, либо удалить
отдельный variant и научить `is_retryable()` разбирать `Error::Sqlx`. Первый
вариант проще для публичного API и сохраняет текущую идею ошибки.

### 4. Retryable SQLSTATE покрыты неполно

Где:

- `crates/rqb/src/error.rs`

Что нашли:

Retryable сейчас фактически ограничены `40001` (serialization failure),
`40P01` (deadlock) и теоретическим `Error::Connection`.

Не хватает типичных retryable Postgres states:

- `57P01` - `admin_shutdown`;
- `57P02` - `crash_shutdown`;
- `57P03` - `cannot_connect_now`.

Риск:

Сервис может не ретраить ошибки, которые обычно считаются временными. В нормальной
продовой эксплуатации это превращается в лишние 5xx и ручной мат в логах.

Что делать:

Добавить mapping этих SQLSTATE в специализированный retryable branch или
учитывать их в `is_retryable()`. Добавить unit tests на каждое состояние.

### 5. Контракт `SearchRequest` и `OpSet` не совпадает с реализацией

Где:

- `crates/rqb/src/request/compile.rs`
- `crates/rqb/src/meta.rs`

Что нашли:

Часть search-операторов проверяет `OpSet`, но часть обходит его:

- `IsNull` / `IsNotNull` не проверяют `OpSet`.
- `Like`, `ILike`, `Contains`, `StartsWith`, `EndsWith` проверяют только
  hardcoded text-like `pg` type.
- Regex family работает по той же логике.

Значит поле с `.json(JsonKind::Text)` и `OpSet::none()` всё равно searchable
через `isNull`, LIKE-family и Regex-family.

Риск:

Metadata выглядит как единый контроллер возможностей, но фактически не всем
управляет. Это либо баг, либо недодокументированная политика. Сейчас читатель
API легко сделает неверный вывод.

Что делать:

Принять явное решение:

- если `OpSet` должен gate-ить всё, добавить проверки для `IsNull`,
  LIKE-family и Regex-family;
- если null/text-pattern operators намеренно разрешаются только через
  `Meta::json`, честно описать это в rustdoc и README.

Для предсказуемости лучше gate-ить через metadata явно, а не через смесь
`json + pg-type`.

### 6. Клиентские LIKE/Regex patterns не ограничены

Где:

- `crates/rqb/src/request/compile.rs`

Что нашли:

`Regex`, `IRegex`, `Like`, `ILike`, `Contains`, `StartsWith`, `EndsWith` принимают
клиентскую строку без лимита длины и сложности. Postgres POSIX regex и
wildcard-heavy ILIKE на больших таблицах могут быть DoS-поверхностью.

Риск:

Один неприятный search payload может устроить дорогой запрос. Особенно весело,
если API публичный, а statement timeout забыли, потому что конечно забыли.

Что делать:

Добавить документацию с требованием statement timeout на API-boundary. Рассмотреть
настройки `SearchRequest` compile policy: max pattern length, запрет regex по
умолчанию, feature/config flag для regex.

## P2: предсказуемость поведения

### 7. Любой raw-фрагмент отключает prepared-statement caching всего запроса

Где:

- `crates/rqb/src/render/raw.rs`
- `crates/rqb/src/built.rs`

Что нашли:

`render_raw` выставляет `cacheable = false`. Это касается любого raw usage:
`raw_expr`, `raw_predicate`, `raw_source`, `raw()`. Даже стабильный raw fragment
с обычными параметрами отключает `persistent` caching для всего `BuiltQuery`.

Риск:

На горячих путях это может быть performance regression, причём тихий. Пользователь
видит корректный SQL и не понимает, почему prepared cache больше не работает.

Что делать:

Добавить явный API для cache policy:

- per-raw-site opt-in типа `raw_expr(...).cacheable()`;
- или `BuiltQuery::set_cacheable(bool)`;
- или отдельный raw constructor для stable raw fragments.

Документировать текущую семантику, если поведение оставляем.

### 8. `rqb-cli` type map слишком мелкий

Где:

- `crates/rqb-cli/src/type_map.rs`
- `crates/rqb-cli/src/introspect.rs`

Что нашли:

Type map - hardcoded match по базовым UDT names и простым `_T` arrays. Не
интроспектятся:

- enums;
- domains;
- ranges через catalog;
- extension types;
- nested arrays.

Unknown и nested array уходят в RawOnly.

Риск:

Для реального Postgres это будет часто: enums, domains, `vector`, PostGIS,
`hstore`, `ltree`, кастомные типы. Silent RawOnly fallback полезен как safety,
но слаб как generated developer experience.

Что делать:

Усилить introspection:

- читать `pg_enum`, `pg_type`, `pg_range`;
- для domains смотреть base type;
- сохранять qualified type name;
- выводить summary unhandled types в CLI output;
- рассмотреть пользовательский type-map TOML для проектов с extension/custom
  types.

### 9. `validate_search_like` использует hardcoded whitelist типов

Где:

- `crates/rqb/src/request/compile.rs`

Что нашли:

LIKE/Regex-family разрешается только для `text`, `varchar`, `bpchar`, `citext`.
Текстовые domains, `name` и часть extension-like типов не поддержаны.

Риск:

Поведение search compile path расходится с реальными Postgres типами и mirrors
ту же проблему, что `rqb-cli` type map.

Что делать:

Либо хранить более богатую capability metadata, либо давать пользователю явно
пометить поле как pattern-searchable независимо от raw `pg` name.

### 10. `Select::count()` silently strips `lock`

Где:

- `crates/rqb/src/execute.rs`

Что нашли:

`build_count()` очищает `order`, `limit`, `offset`, `fetch`, `lock`, потом
оборачивает запрос в count subquery. Удаление `lock` может удивлять.

Риск:

`SELECT ... FOR UPDATE SKIP LOCKED` и count query могут считать разные множества
строк. Пользователь ожидает count "того же запроса", а получает count без lock
semantics.

Что делать:

Документировать явно. Рассмотреть отдельную ошибку/предупреждение при `.count()`
на locked select или отдельный метод с названием, которое честно говорит, что
count игнорирует lock.

### 11. `find_json_meta` делает O(M*N) lookup

Где:

- `crates/rqb/src/request/compile.rs`

Что нашли:

Для каждого predicate обходится весь список fields в `Source`. Для широкой view
и большого filter tree это M*N.

Риск:

Не главный пожар, но дешевый perf debt. На больших generated views и сложных
поисковых запросах будет лишняя работа.

Что делать:

Собрать field index один раз на compile request path: `HashMap<&str, Meta>` или
локальный `BTreeMap` без хранения в `Source`. Глобальный cache через `OnceLock`
сложнее из-за dynamic sources, так что начинать лучше с локального index на
один `apply_search`.

## P3: SemVer и публичный API

### 12. Crate-root surface слишком большая

Где:

- `crates/rqb/src/lib.rs`

Что нашли:

Crate root flat-exports около двух сотен SQL helpers, плюс `prelude` и `dsl`
экспортируют пересекающиеся subset. Это удобно на старте, но тяжело стабилизировать.

Риск:

До 1.0 это ещё терпимо. После 1.0 любой rename/remove/signature change будет
ломать пользователей. Большая поверхность повышает стоимость поддержки и
документации.

Что делать:

Сузить root API:

- в root оставить основные builders/types/macros;
- SQL function helpers держать в `rqb::dsl`;
- `prelude` оставить для типичных service modules, без всего мира SQL helpers;
- breaking change сделать до 1.0.

### 13. Re-export внешних crates протекает в SemVer

Где:

- `crates/rqb/src/lib.rs`

Что нашли:

Root экспортирует `sqlx`, `chrono`, `uuid`, `serde`, `serde_json`.

Риск:

Бамп внешней зависимости может становиться breaking change для `rqb`, даже если
внутренняя логика не поменялась. Особенно неприятно с `sqlx` 0.x.

Что делать:

Убрать root re-exports или спрятать за feature flags. Оставить документацию о
совместимых версиях зависимостей и позволить приложениям импортировать эти crates
напрямую.

### 14. Type safety не надо продавать как compile-time SQL safety

Где:

- `README.md`
- crate-level docs в `crates/rqb/src/lib.rs`
- runtime validation в `crates/rqb/src/expr/validate.rs`,
  `crates/rqb/src/stmt/validate.rs`

Что нашли:

`Field<T>` помогает с bind type и field-to-field comparisons, но operator
legality живёт в metadata и runtime validation. Например, часть выражений
скомпилируется и упадёт только на `.build()`.

Риск:

Неверные ожидания у пользователей. Это не баг кода, но баг обещания. Самое
опасное в библиотеках - не слабое место, а слабое место, проданное как сила.

Что делать:

В README/rustdoc писать честно: `rqb` даёт typed field metadata, typed bind path
и pre-render validation, но не является полной compile-time SQL type system.

## P4: документация и мелкие контракты

### 15. Qualified identifier split по `.` нужно зафиксировать как контракт

Где:

- `crates/rqb/src/ident.rs`

Что нашли:

`write_quoted_qualified` делит строку по всем точкам. Для обычного
`schema.table` это правильно. Для identifier с literal dot внутри имени это
сломает quoting.

Риск:

Мягкий. Сейчас основные входы (`rqb-cli`, `schema!`) генерируют `schema.table`,
так что это скорее ограничение контракта, чем активный баг.

Что делать:

Документировать, что qualified names должны передаваться как dot-separated
Postgres path components, а не как pre-quoted raw identifiers.

### 16. Нужны fuzz/property tests на parsing boundary

Где:

- raw placeholder parser;
- `escaped_like_pattern`;
- `SearchFilterWire` deserialization.

Что нашли:

Именно эти места имеют формат "маленький парсер на границе". Такие штуки часто
ломаются на неожиданных строках, потому что человек читает счастливый путь, а
пользователь приносит мешок мусора.

Риск:

Parser edge cases будут вылезать поздно и раздражающе.

Что делать:

Добавить focused unit tests сначала. Потом рассмотреть fuzz/property tests для
raw placeholder handling и JSON filter deserialization.

## Что уже проверяли

Локально запускались:

```bash
rustup run 1.93.0-x86_64-unknown-linux-gnu cargo test --workspace --all-features
```

Результат: pass. Postgres integration tests были ignored без
`RQB_TEST_DATABASE_URL`.

```bash
rustup run 1.93.0-x86_64-unknown-linux-gnu cargo check --workspace --no-default-features
```

Результат: pass.

```bash
rustup run 1.93.0-x86_64-unknown-linux-gnu cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Результат: pass.

Не запускались:

- Postgres 18 integration suite;
- Docker-backed suite;
- fuzz/property tests.

## Рекомендуемый порядок работ

1. Починить `SearchFilterWire` strictness и добавить ambiguity tests.
2. Починить mapping connection-like `sqlx::Error` и retryable SQLSTATE.
3. Принять решение по `OpSet` для `IsNull`, LIKE-family и Regex-family.
4. Сделать SQL-aware raw placeholder parser.
5. Ввести лимиты/политику для client LIKE/Regex patterns или минимум жёстко
   задокументировать statement timeout.
6. Разобраться с `cacheable` для raw fragments.
7. Усилить `rqb-cli` introspection для enums/domains/ranges/custom types.
8. Сузить public API/re-exports до 1.0.
9. Уточнить docs по partial type safety, `count()` без lock semantics и qualified
   identifier contract.
10. Оптимизировать metadata lookup в `SearchRequest`.

## Короткий вердикт

Кодовая база крепкая: идея правильная, границы в целом взрослые, тесты не для
галочки. Но перед 1.0 надо закрыть P1 и сильно подумать над public API surface.
Иначе проект зайдёт в стабильность с парой неприятных контрактных мин и будет
потом героически разгребать то, что можно было спокойно прибить заранее.
