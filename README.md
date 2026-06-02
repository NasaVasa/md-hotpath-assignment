# md-hotpath-assignment

Это мое решение задачи.

Не менял публичный API библиотеки, потому что на него завязаны тесты. Основная работа была внутри сервиса.

## Что изменил

Главная правка в `src/service.rs`.

В исходной версии состояние было разнесено по трем разным lock-ам: quotes, status и stats. Плюс `run()` делал `tokio::spawn` на каждое событие. Для обычногобекнда это ок, но для market data так делать нельзя.

Я заменил это на единое внутреннее состояние:

```rust
struct Inner {
    symbols: HashMap<Symbol, SymbolState>,
    stats: ServiceStats,
}
```

Теперь по символу хранится и quote, и status. Событие классифицируется явно:

- snapshot применяется всегда и служит recovery point;
- первый incremental может создать состояние;
- `seq == last_seq + 1` применяется;
- `seq <= last_seq` считается duplicate/stale;
- `seq > last_seq + 1` считается gap, символ становится `Stale`, старый quote не затирается.

`run()` теперь просто читает канал по порядку и применяет события одно за другим. Это проще и честнее для такой задачи.

`unsafe` не использовал. Для текущего контракта он тут не нужен.

## CI/CD

Лежит в `.github/workflows/ci.yml`.

В нем отдельные jobs:

- format: `cargo fmt --all -- --check`
- clippy: `cargo clippy --locked --all-targets --all-features -- -D warnings`
- tests: `cargo test --locked --all-targets`
- build: `cargo build --locked --all-targets`
- security audit: `cargo audit`
- aggregate `CI Success`, чтобы удобно видеть общий статус

## AI / MCP

Что было подключено:
- OpenAI Codex
- Context7 для актуальной документации
- GitHub MCP для просмотра логов джоб
- superpowers для более дисциплинированного флоу: brainstorming, debugging, TDD-style checks, verification-before-completion.

Примеры обращений к агенту вынесены в `AGENT_LOG.md`.

## Codex setup

В репозитории также есть локальная настройка Codex:

- `AGENTS.md` с правилами для агента;
- `.codex/hooks.json`;
- `.codex/hooks/post_edit_quality.sh`;
- `.codex/hooks/stop_verification.sh`.

Хуки гоняют быстрые проверки после правок и полный тест гейт в конце работы. Если раст файлы не менялись, они просто пропускают запуск.

## Как проверить локально

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --all-targets
cargo audit
```
