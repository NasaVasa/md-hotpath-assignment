# Rust Market Data Hot Path Assignment

Это тестовое задание для Rust-разработчика на low-latency / HFT-oriented позицию.

В репозитории есть реализация market-data сервиса. Она написана в стиле обычного async backend: Tokio, `mpsc`, `Arc<Mutex<HashMap<...>>>`, `tokio::spawn` на каждое событие, строковые ключи и полный clone snapshot под lock. На умеренной нагрузке такой код может казаться рабочим. Но в нем есть скрытые проблемы.

Ваша задача — улучшить архитектуру и реализацию, сохранив публичный контракт, который используют тесты.

## Что нужно сделать

0. Сделать так, чтобы проходили все публичные тесты:

```bash
cargo test
```

1. Найти проблемные участки кода и исправить их.

2. Подготовить `AGENT_LOG.md`: см. `AGENT_USAGE.md`.

## Бизнес-контракт

## Технические требования

Сохраните публичные методы, используемые тестами:

```rust
MarketDataService::new()
MarketDataService::apply_event(...)
MarketDataService::run(...)
MarketDataService::get_quote(...)
MarketDataService::symbol_status(...)
MarketDataService::snapshot(...)
MarketDataService::stats(...)
```

Интерналы можно менять полностью.

## Unsafe

`unsafe` не запрещён, но не нужен для хорошего решения этого задания.

Непояснённый `unsafe` будет считаться сильным минусом.
