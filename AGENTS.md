# AGENTS.md

## Назначение

Это Rust-библиотека для обработки top-of-book market data в hot-path стиле.
Главный приоритет: корректная per-symbol sequence state machine, затем latency и только потом декоративные улучшения.

## Рабочие правила

- Перед изменениями читать релевантный код и тесты, не делать предположения по памяти.
- Не менять публичный API без явной причины.
- Не делать unrelated refactor: каждая правка должна быть связана с текущей задачей.
- Не менять `.gitignore` только ради локальных отчетов, Codex-файлов или superpowers-артефактов.

## Rust и HFT-правила

- Цены хранятся как fixed-point integer values, не использовать float для финансовой логики.
- Для incremental updates соблюдать строгий sequence contract:
  - первое сообщение по символу может создать состояние;
  - `seq == last_seq + 1` применяется;
  - `seq <= last_seq` считается duplicate/stale и не меняет quote;
  - `seq > last_seq + 1` считается gap и не должен silently advance state;
  - snapshot является recovery boundary.
- В hot path избегать лишних allocations, лишних `clone`, лишних hash lookups и per-event task spawning.
- Сначала correctness state machine, потом latency optimization. Быстрая неверная market-data book бесполезна.
- Не держать несколько независимых locks для одной atomic state transition, если можно обновить состояние под одним lock.

## Проверки

Перед утверждением, что работа готова, запускать:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --all-targets
cargo audit
```

Для диагностического burst smoke можно запускать:

```bash
cargo test --locked --test stress -- --ignored --nocapture
```

## Codex hooks

- Repo-local hooks лежат в `.codex/hooks.json` и `.codex/hooks/`.
- Hooks требуют review/trust в Codex через `/hooks` перед автоматическим запуском.
- `PostToolUse` hook запускает быстрые Rust checks после edit/apply_patch, если есть изменения в Rust/Cargo путях.
- `Stop` hook запускает полный Rust test gate, если есть изменения в Rust/Cargo путях.
- Если нужно проверить hook вручную без dirty Rust-файлов, использовать:

```bash
CODEX_HOOK_FORCE=1 bash .codex/hooks/post_edit_quality.sh
CODEX_HOOK_FORCE=1 bash .codex/hooks/stop_verification.sh
```

## Git

- Коммиты должны быть осмысленными и маленькими.
- Формат commit message: Conventional Commits, например `fix: enforce market data sequence gaps`.
- Перед commit проверять `git status --short` и добавлять только нужные project files.
- Перед push проверять локальные команды из раздела "Проверки".
