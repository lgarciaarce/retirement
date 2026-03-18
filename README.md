# Retirement - Polymarket Crypto Trading Bot

Trading bot for Polymarket 5-minute crypto up/down binary markets. Streams Binance trades for price context and Polymarket orderbooks for signal generation. Pluggable strategy system with simulated execution and portfolio tracking.

## Architecture

```
src/
  main.rs                - Entry point: CLI parsing, tracing init, engine launch
  lib.rs                 - Module declarations
  error.rs               - AppError enum (thiserror)

  config/
    cli.rs               - CLI args (clap derive): --mode, --log-level, --pairs
    settings.rs          - AppConfig, OperationMode

  types/
    market.rs            - CryptoPair, Outcome, AssetInfo, Market
    tick.rs              - PriceTick, TickSource
    orderbook.rs         - OrderbookSnapshot, OrderbookManager, OrderbookEvent
    order.rs             - OrderId, OrderSide, OrderType, OrderRequest, Order, Fill

  sources/
    mod.rs               - PriceSource, OrderbookSource, MarketClient traits
    binance/
      ws.rs              - BinanceWsClient (combined stream, auto-reconnect)
      types.rs           - BinanceTradeEvent, BinanceCombinedStream
    polymarket/
      ws.rs              - PolymarketWsClient (market channel, auto-reconnect)
      rest.rs            - PolymarketRestClient (Gamma API market discovery)
      types.rs           - GammaMarketResponse, PolymarketWsEvent parsing

  strategy/
    mod.rs               - Strategy trait, StrategyContext
    registry.rs          - StrategyRegistry, build_default_strategies()
    spread_logger.rs     - SpreadLoggerStrategy (spread-based signals on tick)
    arb_logger.rs        - ArbLoggerStrategy (up+down mispricing on OB update)

  execution/
    mod.rs               - OrderExecutor trait
    simulated.rs         - SimulatedExecutor (40ms latency, 2-decimal rounding)

  portfolio/
    mod.rs               - PortfolioReader / PortfolioManager traits
    in_memory.rs         - InMemoryPortfolio (positions, balance, pending orders)
    fees.rs              - Polymarket fee model: 0.25 * (p * (1-p))^2

  engine/
    runner.rs            - Engine: event loop, market rolling, strategy dispatch
```

## Data Flow

```
Binance WS ──→ PriceTick ──→ strategy.on_tick()         ──→ Vec<OrderRequest>
                                                                │
Polymarket WS ──→ OrderbookEvent ──→ OB Manager ──→ strategy.on_orderbook_update()
                                                                │
                                                          Engine assigns OrderId
                                                                │
                                                    portfolio.record_pending_order()
                                                                │
                                                    SimulatedExecutor: spawn { sleep 40ms }
                                                                │
                                                          fill_rx.recv()
                                                                │
                                                    portfolio.apply_fill() (balance + fee)
```

## Market Rolling

Markets are 5-minute epoch-based. The engine automatically:

1. Detects epoch boundaries via a timer in the `select!` loop
2. Aborts the old Polymarket WS task
3. Clears stale orderbook state and drains buffered events
4. Re-discovers markets via the Gamma REST API with the new epoch slug
5. Spawns a fresh Polymarket WS subscribed to new asset IDs

Strategies are paused during the first and last 8 seconds of each epoch to allow for market discovery, WS connection, and initial orderbook snapshots.

## Adding a Strategy

1. Create `src/strategy/my_strat.rs` implementing the `Strategy` trait
2. Add `pub mod my_strat;` to `src/strategy/mod.rs`
3. Register in `build_default_strategies()` in `src/strategy/registry.rs`

Strategies implement `on_tick()` for Binance price events and/or `on_orderbook_update()` for Polymarket orderbook events. Both have default no-op implementations.

## Usage

```bash
# Default: all pairs, simulate-live mode, debug logging
cargo run

# Single pair
cargo run -- --pairs btc

# Multiple pairs
cargo run -- --pairs btc,eth

# Override log level
cargo run -- --log-level info

# Fine-grained control via RUST_LOG
RUST_LOG=retirement=debug,reqwest=warn cargo run
```

## Operation Modes

| Mode               | Description                                  |
|---------------------|----------------------------------------------|
| `simulate-live`     | Live feeds, simulated execution (default)    |
| `live`              | Live trading (not yet implemented)           |
| `simulate-persisted`| Replay from saved data (not yet implemented) |

## Supported Pairs

| Pair | Binance Symbol | Polymarket Slug Prefix |
|------|----------------|------------------------|
| btc  | btcusdt        | btc-updown             |
| eth  | ethusdt        | eth-updown             |
| sol  | solusdt        | sol-updown             |
| xrp  | xrpusdt        | xrp-updown             |
| doge | dogeusdt       | doge-updown            |

## Build Requirements

- Rust 2024 edition (1.85+)
- On Windows with `stable-x86_64-pc-windows-gnu` toolchain: WinLibs MinGW with `dlltool.exe` must be in PATH
