# CLAUDE.md

## Build & Test

```bash
cargo clippy            # lint — must pass with zero warnings
cargo test              # run all tests
cargo run -- --pairs btc --log-level debug   # quick smoke test with one pair
```

Windows GNU toolchain: WinLibs MinGW `dlltool.exe` must be in PATH to link.

## Project Overview

Polymarket crypto up/down binary-market trading bot. 5-minute epoch markets that auto-roll. Two live data sources (Binance price ticks, Polymarket orderbook via WS), pluggable strategy system, simulated execution with 40ms latency, in-memory portfolio with Polymarket fee model.

## Key Architecture Decisions

- **Strategies are sync** (`fn on_tick`, `fn on_orderbook_update`) — pure decision functions, no async I/O. This keeps them testable and avoids lifetime issues with borrows in `StrategyContext`.
- **Two strategy hooks**: `on_tick` fires on Binance price events, `on_orderbook_update` fires on Polymarket OB events. Both have default empty implementations — a strategy implements only what it needs.
- **Market rolling**: The engine auto-rolls markets at 5-minute epoch boundaries. Old WS tasks are aborted, orderbook state is cleared, new markets are discovered via REST, and a fresh WS is spawned. Strategies are paused during the buffer window (first/last N seconds of each epoch).
- **Orderbook reconciliation**: `PriceChange` events carry optional `best_bid`/`best_ask` from the exchange. After applying level updates, the book is reconciled against these values to remove phantom levels from missed cancellation events.
- **Fee model**: Price-dependent — `0.25 * (p * (1-p))^2` applied to notional (price * size). Not a flat rate.
- **Portfolio traits**: `PortfolioReader` (read-only, used by strategies) / `PortfolioManager` (mutable, used by engine). `InMemoryPortfolio` implements both. Trait-based for future swap to live/persistent implementations.
- **Execution trait**: `OrderExecutor` is async. `SimulatedExecutor` spawns a task that sleeps 40ms then sends a `Fill`. Non-blocking. Designed so a live executor can slot in later.

## Module Guide

| Module | Purpose |
|--------|---------|
| `engine/runner.rs` | Main event loop (`tokio::select!`), market discovery, epoch rolling, strategy dispatch, order processing |
| `strategy/` | `Strategy` trait + `StrategyRegistry`. Add new strategies here |
| `execution/` | `OrderExecutor` trait. `SimulatedExecutor` with 40ms latency and 2-decimal rounding |
| `portfolio/` | Position/balance tracking + fee calculation |
| `types/order.rs` | `OrderRequest`, `Order`, `Fill`, `OrderId`, `OrderSide`, `OrderType` |
| `types/orderbook.rs` | `OrderbookManager` (multi-asset state), `OrderbookSnapshot` (single-asset book with reconciliation) |
| `sources/` | `PriceSource`, `OrderbookSource`, `MarketClient` traits + Binance/Polymarket implementations |

## Conventions

- Error handling: `AppError` enum with `thiserror`, all fallible functions return `crate::error::Result<T>`
- Logging: `tracing` crate. Use `trace` for per-event data, `debug` for strategy signals and fills, `info` for lifecycle events, `warn`/`error` for problems
- Async: `tokio` runtime, `async-trait` for async trait methods, MPSC channels for inter-task communication
- Polymarket WS events: parsed via serde tagged enum (`PolymarketWsEvent`), unknown fields silently ignored
- Prices are `f64`, Polymarket prices are probabilities 0.01–0.99, fills are rounded to 2 decimal places
