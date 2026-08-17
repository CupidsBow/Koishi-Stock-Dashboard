# Stock Dashboard

A full-stack web application for viewing Chinese A-stock candlestick charts with dual-strategy buy/sell signals. Built with:

- **Backend**: Rust + Axum + Polars + ndarray + SQLx (PostgreSQL) — all indicators and factor computation in pure Rust
- **Frontend**: Vite + React 19 + TypeScript + lightweight-charts

## Project Structure

```
stock-dashboard/
├── stock-backend/              # Rust HTTP API server (port 3000)
│   ├── Cargo.toml              # Dependencies: axum, polars, ndarray, sqlx, reqwest…
│   └── src/
│       ├── main.rs             # Axum router, server entry, static file serving, CORS
│       ├── models.rs           # Core domain types: Candle, AlphaScore, Signal, FactorEval…
│       ├── controllers/
│       │   ├── health.rs       # GET /api/health
│       │   ├── indicators.rs   # GET /api/indicators — main data endpoint
│       │   └── search.rs       # GET /api/search (Eastmoney suggest API)
│       └── services/
│           ├── stock.rs        # Data layer: Tencent kline + Eastmoney search via reqwest
│           ├── cache.rs        # PostgreSQL JSONB cache with market-aware TTL
│           ├── db.rs           # Connection pool, health check, cache table migration
│           ├── indicators/     # 7 traditional indicators (pure Rust)
│           │   ├── bollinger.rs    # Bollinger Bands (20)
│           │   ├── keltner.rs      # Keltner Channels (20/10)
│           │   ├── macd.rs         # MACD (12/26/9)
│           │   ├── kdj.rs          # KDJ (9)
│           │   ├── adx.rs          # ADX (14)
│           │   ├── rsi.rs          # RSI (2)
│           │   ├── regime.rs       # Market regime classifier (5 states)
│           │   └── signals.rs      # CTA rule-based buy/sell signals (KDJ+MACD+BB+KC)
│           ├── expression/     # Factor expression engine (Stage 1 of Alpha pipeline)
│           │   ├── parser.rs       # Recursive-descent parser + AST validator
│           │   ├── registry.rs     # 12 built-in factor definitions (4 categories)
│           │   └── compute.rs      # Numeric factor evaluation (rolling stats, 12 hard-coded formulas)
│           ├── evaluation/     # IC/IR evaluation (Stage 2)
│           │   ├── ic.rs           # Rolling IC: Pearson correlation with forward N-day returns
│           │   └── screen.rs       # Factor validity screening: |IR|>0.3 or |IC|>0.02
│           ├── prediction/     # Normalization + synthesis (Stage 3)
│           │   ├── normalize.rs    # Winsorize at 1%/99% → z-score → direction adjustment
│           │   └── synthesize.rs   # IC-weighted multi-factor score → AlphaScore
│           └── signals/        # Alpha signal generation (Stage 4)
│               ├── quantile.rs     # Quantile-threshold signals (P80/Buy, P20/Sell…)
│               ├── reversal.rs     # Zero-axis crossover signals (α turns +/−)
│               └── divergence.rs   # Price-score divergence (top/bottom divergence)
├── frontend/                   # React SPA (Vite dev server → proxied to :3000)
│   ├── vite.config.ts
│   ├── index.html
│   └── src/
│       ├── main.tsx            # React entry
│       ├── App.tsx             # Root layout, strategy toggle, signal toggles, localStorage
│       ├── App.css             # Global + component styles (dark/light theme)
│       ├── api.ts              # Backend API client (fetchIndicators, searchStocks)
│       ├── types.ts            # Shared TypeScript interfaces (Candle, Signal, AlphaScore…)
│       └── components/
│           ├── StockSearch.tsx     # Stock code/name search (debounced)
│           ├── StockList.tsx       # Sidebar watch list (add/remove/reorder/drag)
│           └── StockChart.tsx      # lightweight-charts: K-line + BB/KC/MACD/KDJ + signal markers
├── build-deploy.sh             # Production build + tarball packaging
├── start.sh                    # One-command dev server launcher (backend + frontend)
└── README.md
```

## Quick Start

### Prerequisites

- Rust toolchain (≥ 1.85 for edition 2024)
- Node.js (≥ 18)
- npm
- PostgreSQL (for K-line data caching)

### One-Command Dev Start

```bash
./start.sh
# → Starts backend on :3000, frontend on :5173
# → Ctrl+C to stop both
```

### Manual Development

```bash
# 1. Set up PostgreSQL
# Create a database and set DATABASE_URL in stock-backend/.env:
#   DATABASE_URL=postgres://user:password@localhost/stock_db

# 2. Start the backend
cd stock-backend
cargo run
# → Listening on http://0.0.0.0:3000

# 3. Start the frontend dev server (in another terminal)
cd frontend
npm install
npm run dev
# → Vite dev server, API calls proxy to localhost:3000
```

Then open the Vite dev URL (usually http://localhost:5173).

### Production Build

```bash
./build-deploy.sh
# → Build backend (release), build frontend (dist/)
# → Assemble deploy-pkg/ with start.sh, keepalive.sh
# → Create stock-dashboard.tar.gz (≈ 8.6 MB)
```

Deploy to server:
```bash
scp stock-dashboard.tar.gz user@server:/opt/
ssh user@server
cd /opt && tar xzf stock-dashboard.tar.gz
cd stock-dashboard/deploy-pkg
cp .env.example .env     # edit DATABASE_URL
./start.sh               # backend serves frontend/dist as static files on :3000
```

## API Endpoints

| Method | Path              | Params                                                                  | Description                |
|--------|-------------------|-------------------------------------------------------------------------|----------------------------|
| GET    | `/api/health`     | —                                                                       | Health check               |
| GET    | `/api/search`     | `keyword` (string)                                                      | Search A-share stocks by code or name |
| GET    | `/api/indicators` | `symbol`, `days` (default 400), `strategy` (default/alpha/hybrid), `forward` (default 5), `quantile`, `reversal`, `divergence` | Daily candles + all indicators + buy/sell signals |

### `/api/indicators` Response

| Field             | Type                          | Description                                            |
|-------------------|-------------------------------|--------------------------------------------------------|
| `candles`         | `Candle[]`                    | Daily OHLCV (前复权)                                    |
| `bollinger`       | `BollingerPoint[]` (nullable) | Upper / middle / lower bands (20)                       |
| `keltner`         | `KeltnerPoint[]` (nullable)   | Upper / middle / lower channels (20/10)                 |
| `macd`            | `MacdPoint[]` (nullable)      | DIF / DEA / BAR (12/26/9)                               |
| `kdj`             | `KdjPoint[]` (nullable)       | K / D / J (9)                                           |
| `adx`             | `AdxPoint[]` (nullable)       | ADX / +DI / -DI (14)                                    |
| `rsi`             | `number[]` (nullable)         | RSI values (2)                                          |
| `regime`          | `string`                      | Market regime (窄幅整理 / 震荡市 / 单边牛市 / 单边熊市 / 趋势衰竭) |
| `signals`         | `Signal[]`                    | CTA rule-based buy/sell signals with price + reason + PnL |
| `factor_evals`    | `FactorEval[]`                | Per-factor IC/IR/weight evaluation (alpha/hybrid mode)  |
| `factor_scores`   | `AlphaScore[]` (nullable)     | Composite alpha scores per time point                   |
| `signals_v2`      | `Signal[]`                    | Alpha model buy/sell signals (paired, with PnL)         |

## Dual-Strategy System

The dashboard provides two trading signal strategies, switchable in the UI:

### Strategy 1: CTA Rules (传统规则)

Based on cross-confirmation of 4 traditional indicators within a 3-bar window:

| Confirmation     | Signal | Meaning              |
|------------------|--------|----------------------|
| KDJ↑ + MACD↑     | Buy    | 金叉双重确认           |
| KDJ↑ + 支撑       | Buy    | KDJ金叉 触及布林/肯特纳下轨 |
| MACD↑ + 支撑      | Buy    | MACD金叉 触及支撑位    |
| KDJ↓ + MACD↓     | Sell   | 死叉双重确认           |
| KDJ↓ + 压力       | Sell   | KDJ死叉 触及上轨       |
| MACD↓ + 压力      | Sell   | MACD死叉 触及压力位    |

Additional signals: KDJ J-line oversold (< 0) / overbought (> 100), BB squeeze + volume breakout.

**Signal pairing**: Buys and Sells are chronologically matched — each Sell closes all currently-open Buy positions. Unclosed buys are marked "持仓中".

Signal `reason` format examples:
- `买(kdj↑ & macd↑) @10.50→11.20(+6.7%)`
- `卖(kdj↓ & macd↓)`

### Strategy 2: Alpha Factor Model (因子Alpha)

A 4-stage quantitative pipeline:

#### Stage 1 — Raw Factor Computation

12 built-in factors across 4 categories, computed from OHLCV data in pure Rust:

| Category    | Factor     | Expression                                      | Direction |
|-------------|------------|-------------------------------------------------|-----------|
| **Momentum** | `ret_5d`   | `Delta(Close,5)/Delay(Close,5)` — 5-day return  | +1 |
|             | `ret_20d`  | `Delta(Close,20)/Delay(Close,20)` — 20-day return | +1 |
|             | `macd_dif` | `Ts_Mean(Close,12)-Ts_Mean(Close,26)` — MACD diff | +1 |
|             | `ma_disp_20` | `(Close-Ts_Mean(Close,20))/Ts_Mean(Close,20)` — MA dispersion | +1 |
| **Volatility** | `atr_pct` | `Ts_Mean(Abs(Delta(Close,1)),14)/Close` — ATR ratio | −1 |
|             | `bb_width`  | `2*Ts_Std(Close,20)/Ts_Mean(Close,20)` — BB width | 0 (auto) |
|             | `hl_vol`    | `Ts_Std(High/Low-1,10)` — high-low volatility    | −1 |
|             | `ret_std`   | `Ts_Std(Delta(Close,1)/Delay(Close,1),20)` — return std | −1 |
| **Volume**   | `vol_ratio` | `Volume/Ts_Mean(Volume,5)` — volume ratio        | 0 (auto) |
|             | `vol_trend` | `Delta(Ts_Mean(Volume,5),5)/Ts_Mean(Volume,5)` — volume trend | +1 |
| **Trend**    | `ma_cross`  | `Ts_Mean(Close,5)-Ts_Mean(Close,20)` — MA cross  | +1 |
|             | `slope_10`  | `(Close-Delay(Close,10))/Delay(Close,10)` — 10-day slope | +1 |

Direction: +1 = higher is bullish, −1 = higher is bearish, 0 = auto-detect from IC sign.

#### Stage 2 — Rolling IC Evaluation

- **Target**: Forward N-day return (default N=5): `fwd_ret[t] = (close[t+N] − close[t]) / close[t]`
- **Rolling IC**: Pearson correlation between each factor and forward return over a 60-day window
- **Factor screening**: A factor is valid if `|IR| > 0.3` or `|IC| > 0.02`
- **Weight assignment**: Valid → `weight = |IC_mean|`; Invalid → `weight = 0`

#### Stage 3 — Normalization & Synthesis

1. **Winsorize** at 1%/99% quantiles — clip extreme outliers
2. **Z-score**: `(x − μ) / σ` per factor column
3. **Direction adjustment**: multiply by direction sign (1 or −1)
4. **IC-weighted synthesis**: `AlphaScore.total = Σ(weight × z_score)` summed across all valid factors, also broken down by category (momentum/volatility/volume/trend)

A higher `total` value = stronger bullish expectation.

#### Stage 4 — BS Signal Generation

Three complementary signal types (individually toggleable in the UI):

**A. Quantile Threshold** (120-bar historical percentiles)

| Condition                                    | Signal       |
|----------------------------------------------|--------------|
| `total > P80` AND score rising (5-bar)       | Strong Buy   |
| `total > P60` AND `total > 0`                | Buy          |
| `total < P20` AND score falling (5-bar)      | Strong Sell  |
| `total < P40` AND `total < 0`                | Sell         |

**B. Zero-Axis Reversal**

| Condition                                                    | Signal |
|--------------------------------------------------------------|--------|
| `prev.total ≤ 0 ∧ cur.total > 0` AND rising 3 consecutive bars | Buy (基本面改善) |
| `prev.total ≥ 0 ∧ cur.total < 0` AND falling 3 consecutive bars | Sell (基本面恶化) |

**C. Price-Factor Divergence** (20-bar lookback)

| Condition                                                                  | Signal |
|----------------------------------------------------------------------------|--------|
| Price at new 20-day high, Alpha NOT at new high (< 0.9×max), Alpha falling 5 bars | Sell (顶背离) |
| Price at new 20-day low, Alpha NOT at new low (> 1.1×min), Alpha rising 5 bars   | Buy (底背离) |

Signal pairing and PnL calculation is applied to all Alpha signals, matching each Sell to all open Buys.

## Technical Indicators

All indicators are computed **in pure Rust** on the backend:

| Indicator          | Period  | Output                  |
|--------------------|---------|-------------------------|
| Bollinger Bands    | 20      | upper, middle, lower    |
| Keltner Channels   | 20/10   | upper, middle, lower    |
| MACD               | 12/26/9 | dif, dea, bar           |
| KDJ                | 9       | k, d, j                 |
| ADX                | 14      | adx, +di, −di           |
| RSI                | 2       | single value            |

### Market Regime Classification

Requires ≥ 60 days of data. Based on ADX value + Bollinger bandwidth trend:

| Regime   | ADX         | BB Width     |
|----------|-------------|--------------|
| 窄幅整理  | < 20        | declining    |
| 震荡市    | 20–25       | stable       |
| 单边牛市  | > 25        | expanding (close > BB middle) |
| 单边熊市  | > 25        | expanding (close < BB middle) |
| 趋势衰竭  | > 25        | contracting  |

## Data Sources

| Data            | Source                                | Method          |
|-----------------|---------------------------------------|-----------------|
| Daily K-line    | Tencent Finance (`web.ifzq.gtimg.cn`) | reqwest GET     |
| Stock search    | Eastmoney (`searchapi.eastmoney.com`) | reqwest GET     |

All data is fetched via `reqwest` directly — no Python, no FFI, no third-party SDK. Pre-adjusted (前复权 `qfq`) daily data is used for all calculations.

## Caching

K-line data is cached in PostgreSQL (`candles_cache` table, JSONB column). Cache TTL is market-aware:

- **Market open** (Mon–Fri, 9:30–15:00 CST): 5 minutes
- **Market closed**: 2 hours

Cache misses fall through to the Tencent API, then asynchronously write back to cache.

## Frontend Features

- **Strategy toggle**: Switch between CTA Rules and Factor Alpha in the UI
- **Signal type toggles**: In Alpha mode, individually enable/disable quantile, reversal, and divergence signals
- **Watch list**: Drag-to-reorder sidebar, persisted in `localStorage`
- **Lazy history loading**: Scroll to left edge → auto-loads more historical data (800 → 1600 → 3200 → 10000 days)
- **Signal markers**: Compact "B"/"S" dots on the K-line chart, hover to see full signal reason
- **Candle tinting**: Confirmed signal bars are tinted deeper red/green
- **Factor dashboard**: Shows valid factor count, current Alpha score, and cumulative PnL
- **Three-pane layout**: Main chart (candles + BB/KC) → KDJ sub-pane → MACD sub-pane
- **Dark/light mode**: Follows system `prefers-color-scheme`

## Key Parameters

| Parameter              | Default | Description                              |
|------------------------|---------|------------------------------------------|
| Forward period         | 5 days  | IC target look-ahead horizon             |
| IC rolling window      | 60 days | Correlation window for factor evaluation |
| Quantile reference     | 120 bars | Historical score percentile window      |
| Divergence lookback    | 20 bars | Price-score divergence detection window  |
| Winsorize quantiles    | 1%/99%  | Outlier clipping thresholds              |
| Factor validity check  | \|IR\|>0.3 or \|IC\|>0.02 | Significance threshold         |
| ADX period             | 14      | ADX/+DI/−DI smoothing period             |
| Cache TTL (market open) | 5 min   | Data freshness during trading hours      |
| Cache TTL (market closed) | 2 hours | Data freshness outside trading hours   |

## Lines of Code

| Layer              | Lines  |
|--------------------|--------|
| Backend Rust       | 3,867  |
| Frontend TS/TSX/CSS | 2,066  |
| Build scripts      | 240    |
| **Total**          | **6,173** |