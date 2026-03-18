# Retirement - Polymarket Crypto Trading Bot

Trading bot for Polymarket 5m/15m crypto updown markets. Integrates Binance WebSocket for low-latency price feeds and Polymarket APIs (REST + WebSocket) for market discovery and orderbook data.

## Architecture

```
src/
  main.rs          - Entry point: CLI parsing, tracing init, engine launch
  lib.rs           - Module declarations
  error.rs         - AppError enum (thiserror)
  config/
    cli.rs         - CLI args (clap derive): --mode, --log-level, --pairs
    settings.rs    - AppConfig, OperationMode, TokenPairConfig
  types/
    market.rs      - Market domain type
    tick.rs        - PriceTick, TickSource
    orderbook.rs   - OrderbookEvent, PriceLevel
  sources/
    mod.rs         - PriceSource, OrderbookSource, MarketClient traits
    binance/
      types.rs     - BinanceTradeEvent, BinanceCombinedStream
      ws.rs        - BinanceWsClient (combined stream, auto-reconnect)
    polymarket/
      types.rs     - GammaMarketResponse, PolymarketWsEvent
      rest.rs      - PolymarketRestClient (GET /markets/slug/{slug})
      ws.rs        - PolymarketWsClient (market channel subscription)
  engine/
    runner.rs      - Engine: orchestrates sources, channels, tasks
```

## Operation Modes

- **`simulate-live`** (default) - Connects to live Binance + Polymarket feeds, logs all data, no trading
- **`live`** - Live trading (not yet implemented)
- **`simulate-persisted`** - Replay from saved data (not yet implemented)

## Usage

```bash
# Default: all pairs, simulate-live mode, debug logging
cargo run

# Single pair
cargo run -- --pairs btc

# Multiple specific pairs
cargo run -- --pairs btc,eth

# Override log level
cargo run -- --log-level info

# Use RUST_LOG env var for fine-grained control
RUST_LOG=retirement=debug,reqwest=warn cargo run
```

## Build Requirements

- Rust 2024 edition (1.94+)
- On Windows with `stable-x86_64-pc-windows-gnu` toolchain: MinGW with `dlltool.exe` must be in PATH

## Supported Token Pairs

| Name | Binance Symbol | Polymarket Slug Prefix                  |
|------|----------------|-----------------------------------------|
| btc  | btcusdt        | eth-updown-                             |
| eth  | ethusdt        | eth-updown-                             |
| sol  | solusdt        | sol-updown-                             |
| xrp  | xrpusdt        | xrp-updown-                             |
| doge | dogeusdt       | xrp-updown-                             |
