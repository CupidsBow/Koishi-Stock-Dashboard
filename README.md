# Stock Dashboard

A web application for viewing Chinese A-stock candlestick charts. Built with:

- **Backend**: Rust + Axum + akshare (stock data provider)
- **Frontend**: Vite + React 19 + TypeScript + lightweight-charts

## Project Structure

```
stock-dashboard/
├── stock-backend/      # Rust HTTP API server (port 3000)
│   └── src/
│       ├── main.rs     # Server entry, routes, static file serving
│       ├── api.rs      # Request handlers (GET /api/candles, /api/search)
│       └── stock.rs    # akshare data-fetching layer
├── frontend/           # React SPA (Vite dev server)
│   └── src/
│       ├── App.tsx                  # Root layout + state management
│       ├── api.ts                   # Backend API client
│       ├── types.ts                 # Shared TypeScript types
│       └── components/
│           ├── StockSearch.tsx      # Stock code/name search input
│           ├── StockList.tsx        # Sidebar watch list
│           └── StockChart.tsx       # lightweight-charts candlestick chart
└── README.md
```

## Quick Start

### Prerequisites

- Rust toolchain (>= 1.85 for edition 2024)
- Node.js (>= 18)
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
npm run build

# Start the backend (it serves frontend/dist as static files)
cd ../stock-backend
cargo run --release
# → Full app available at http://localhost:3000
```

## API Endpoints

| Method | Path            | Params                    | Description                      |
| ------ | --------------- | ------------------------- | -------------------------------- |
| GET    | `/api/search`   | `keyword` (string)        | Search stocks by code or name    |
| GET    | `/api/candles`  | `symbol` (string), `days` (int, default 60) | Get OHLCV candlestick data |

## Notes

- Stock data is sourced from public financial data APIs via the `akshare` crate.
- The watch list is persisted in the browser's `localStorage`.
- The frontend dev server proxies `/api` requests to the backend — make sure the backend is running on port 3000 before starting the frontend.