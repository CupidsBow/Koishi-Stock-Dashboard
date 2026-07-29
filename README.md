# Stock Dashboard

A full-stack web application for viewing Chinese A-stock candlestick charts with real-time technical indicators and buy/sell signals. Built with:

- **Backend**: Rust + Axum + reqwest (direct HTTP to Tencent/Eastmoney APIs)
- **Frontend**: Vite + React 19 + TypeScript + lightweight-charts

## Project Structure

```
stock-dashboard/
├── stock-backend/          # Rust HTTP API server (port 3000)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # Axum router, server entry, static file serving
│       ├── api.rs          # Request handlers + DTOs (Candle, IndicatorsResponse, etc.)
│       ├── stock.rs        # Data layer: Tencent kline + Eastmoney search via reqwest
│       └── indicators.rs   # 6 technical indicators + market regime + buy/sell signals
├── frontend/               # React SPA (Vite dev server → proxied to :3000)
│   ├── vite.config.ts
│   ├── index.html
│   └── src/
│       ├── main.tsx                    # React entry
│       ├── App.tsx                     # Root layout + watch list state (localStorage)
│       ├── App.css                     # Global + component styles
│       ├── api.ts                      # Backend API client (fetchIndicators, searchStocks)
│       ├── types.ts                    # Shared TypeScript interfaces
│       └── components/
│           ├── StockSearch.tsx         # Stock code/name search (debounced)
│           ├── StockList.tsx           # Sidebar watch list (add/remove/select)
│           └── StockChart.tsx          # lightweight-charts: K-line + Bollinger/Keltner/MACD/KDJ + signal labels
└── README.md
```

## Quick Start

### Prerequisites

- Rust toolchain (≥ 1.85 for edition 2024)
- Node.js (≥ 18)
- npm

### Development

```bash
# 1. Start the backend
cd stock-backend
cargo run
# → Listening on http://0.0.0.0:3000

# 2. Start the frontend dev server (in another terminal)
cd frontend
npm install
npm run dev
# → Vite dev server, API calls proxy to localhost:3000
```

Then open the Vite dev URL (usually http://localhost:5173).

### Production Build

```bash
# Build the frontend
cd frontend
npm run build        # → dist/

# Start the backend (serves frontend/dist as static files)
cd ../stock-backend
cargo run --release
# → Full app at http://localhost:3000
```

## API Endpoints

| Method | Path               | Params                          | Description                                                    |
|--------|--------------------|---------------------------------|----------------------------------------------------------------|
| GET    | `/api/health`      | —                               | Health check                                                    |
| GET    | `/api/search`      | `keyword` (string)              | Search A-share stocks by code or name (Eastmoney suggest API)   |
| GET    | `/api/indicators`  | `symbol` (string), `days` (int, default 400) | Daily candles + all technical indicators + buy/sell signals |

### `/api/indicators` Response

Returns `IndicatorsResponse`:

| Field        | Type                          | Description                          |
|--------------|-------------------------------|--------------------------------------|
| `candles`    | `Candle[]`                    | Daily OHLCV (前复权)                 |
| `bollinger`  | `BollingerPoint[]` (nullable) | Upper / middle / lower bands (20)    |
| `keltner`    | `KeltnerPoint[]` (nullable)   | Upper / middle / lower channels       |
| `macd`       | `MacdPoint[]` (nullable)      | DIF / DEA / BAR (12/26/9)           |
| `kdj`        | `KdjPoint[]` (nullable)       | K / D / J (9)                        |
| `adx`        | `AdxPoint[]` (nullable)       | ADX / +DI / -DI (14)                |
| `rsi`        | `number[]` (nullable)         | RSI values (2)                       |
| `regime`     | `string`                      | Market regime (中文): 窄幅整理 / 震荡市 / 单边牛市 / 单边熊市 / 趋势衰竭 |
| `signals`    | `Signal[]`                    | Buy/sell signals with price + reason |

## Technical Indicators

All indicators are computed **in pure Rust** on the backend:

| Indicator          | Period | Output                  |
|---------------------|--------|-------------------------|
| Bollinger Bands     | 20     | upper, middle, lower    |
| Keltner Channels    | 20/10  | upper, middle, lower    |
| MACD                | 12/26/9| dif, dea, bar           |
| KDJ                 | 9      | k, d, j                 |
| ADX                 | 14     | adx, +di, -di           |
| RSI                 | 2      | single value            |

### Buy/Sell Signal System

Signals are generated through a **3-layer cross-confirmation** pipeline:

1. **Per-indicator signals**: KDJ golden/death crosses, MACD DIF/DEA crosses, support/resistance at Bollinger/Keltner bands
2. **Cross-confirmation** (within 3-bar window): KDJ↑MACD↑, KDJ↑支撑, MACD↑支撑 (and their sell counterparts)
3. **Signal merging**: buys and sells are chronologically paired, PnL calculated, unmatched buys marked as "持仓中"

Additional signals: KDJ J-line oversold (< 0) / overbought (> 100).

Signal `reason` format examples:
- `买(kdj↑macd↑ & kdj↑支撑) @10.50→11.20(+6.7%)`
- `卖(kdj↓macd↓)`

## Data Sources

| Data            | Source                                        |
|-----------------|-----------------------------------------------|
| Daily K-line    | Tencent Finance (`web.ifzq.gtimg.cn`)          |
| Stock search    | Eastmoney (`searchapi.eastmoney.com`)          |

All data is fetched via `reqwest` directly — no Python, no FFI, no third-party SDK.

## Notes

- The watch list is persisted in the browser's `localStorage` under key `stock-dashboard:watchlist`.
- Pre-adjusted (前复权 `qfq`) daily data is used for all indicator calculations.
- Market regime classification requires ≥ 60 days of data.
- Chart supports lazy-loading more historical data when scrolling to the left edge (up to 10,000 days).
- The frontend dev server proxies `/api` requests to `localhost:3000`.

## Lines of Code

| Layer         | Lines |
|---------------|-------|
| Backend Rust  | 1,136 |
| Frontend TS/TSX | 1,072 |
| CSS           | 290   |
| Config        | 28    |
| **Total**     | **2,526** |