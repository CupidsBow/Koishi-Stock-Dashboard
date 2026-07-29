import { useState, useCallback, useEffect, useRef, useMemo } from "react";
import { fetchIndicators } from "./api";
import type { Candle, StockItem, StockInfo, IndicatorsResponse } from "./types";
import StockSearch from "./components/StockSearch";
import StockList from "./components/StockList";
import StockChart from "./components/StockChart";
import "./App.css";

const STORAGE_KEY = "stock-dashboard:watchlist";
const INITIAL_DAYS = 400;
const LOAD_MORE_DAYS = [800, 1600, 3200, 10000];

function loadStocks(): StockItem[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function saveStocks(stocks: StockItem[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(stocks));
}

/** Merge older (more days) into newer (already on screen).
 *  Uses older's indicator values for the ENTIRE merged dataset because
 *  they were computed with a larger lookback window (fewer nulls, more converged). */
function mergeIndicators(older: IndicatorsResponse, newer: IndicatorsResponse): IndicatorsResponse {
  const newerTimes = new Set(newer.candles.map((c) => c.time));

  // Split older's indices into "extra" (not in newer) and "overlap" (in newer)
  const extraIdx: number[] = [];
  const overlapIdx: number[] = [];
  for (let i = 0; i < older.candles.length; i++) {
    if (!newerTimes.has(older.candles[i].time)) {
      extraIdx.push(i);
    } else {
      overlapIdx.push(i);
    }
  }

  // Candles: prepend extra history, keep newer's values for overlap (same data)
  const candles = [...extraIdx.map((i) => older.candles[i]), ...newer.candles];

  // indicators: use OLDER's values everywhere — larger lookback = fewer nulls
  const pickOlder = <T,>(arr: (T | null)[]) => [
    ...extraIdx.map((i) => arr[i]),
    ...overlapIdx.map((i) => arr[i]),
  ];

  // signals: prepend older's extra-history signals not already in newer
  const signals = [
    ...older.signals.filter((s) => !newerTimes.has(s.time)),
    ...newer.signals,
  ];

  return {
    candles,
    bollinger: pickOlder(older.bollinger),
    keltner:  pickOlder(older.keltner),
    macd:     pickOlder(older.macd),
    kdj:      pickOlder(older.kdj),
    adx:      pickOlder(older.adx || []),
    rsi:      pickOlder(older.rsi || []),
    regime:   older.regime || newer.regime || "震荡市",
    signals,
  };
}

export default function App() {
  const [stocks, setStocks] = useState<StockItem[]>(loadStocks);
  const [activeSymbol, setActiveSymbol] = useState<string | null>(null);
  const [indicators, setIndicators] = useState<IndicatorsResponse | null>(null);
  const [candles, setCandles] = useState<Candle[]>([]);
  const [loadingChart, setLoadingChart] = useState(false);
  const [chartError, setChartError] = useState<string | null>(null);
  const [loadingMore, setLoadingMore] = useState(false);

  const loadStepRef = useRef(0);
  const loadingMoreRef = useRef(false);

  useEffect(() => {
    saveStocks(stocks);
  }, [stocks]);

  // Initial load
  useEffect(() => {
    if (!activeSymbol) {
      setCandles([]);
      setIndicators(null);
      setChartError(null);
      loadStepRef.current = 0;
      return;
    }
    let cancelled = false;
    setLoadingChart(true);
    setChartError(null);
    loadStepRef.current = 0;
    fetchIndicators(activeSymbol, INITIAL_DAYS)
      .then((data) => {
        if (!cancelled) {
          setCandles(data.candles);
          setIndicators(data);
          setLoadingChart(false);
        }
      })
      .catch((e) => {
        if (!cancelled) {
          setChartError(e instanceof Error ? e.message : "Failed to load data");
          setCandles([]);
          setIndicators(null);
          setLoadingChart(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [activeSymbol]);

  // Lazy load
  const handleReachLeftEdge = useCallback(async () => {
    if (!activeSymbol || loadingMoreRef.current) return;
    const nextStep = loadStepRef.current;
    if (nextStep >= LOAD_MORE_DAYS.length) return;

    const days = LOAD_MORE_DAYS[nextStep];
    loadingMoreRef.current = true;
    setLoadingMore(true);

    try {
      const data = await fetchIndicators(activeSymbol, days);
      setIndicators((prev) => {
        if (!prev) return data;
        return mergeIndicators(data, prev);
      });
      setCandles((prev) => {
        const seen = new Set(prev.map((c) => c.time));
        return [...data.candles.filter((c) => !seen.has(c.time)), ...prev];
      });
      loadStepRef.current = nextStep + 1;
    } catch {
      // silently fail on lazy load
    } finally {
      loadingMoreRef.current = false;
      setLoadingMore(false);
    }
  }, [activeSymbol]);

  const handleAddStock = useCallback(
    (stock: StockInfo) => {
      setStocks((prev) => {
        if (prev.some((s) => s.symbol === stock.symbol)) return prev;
        const item: StockItem = { ...stock, addedAt: Date.now() };
        return [...prev, item];
      });
      setActiveSymbol(stock.symbol);
    },
    []
  );

  const handleRemoveStock = useCallback((symbol: string) => {
    setStocks((prev) => prev.filter((s) => s.symbol !== symbol));
    setActiveSymbol((current) => (current === symbol ? null : current));
  }, []);

  const activeStock = stocks.find((s) => s.symbol === activeSymbol);

  const totalClosedPnl = useMemo(() => {
    if (!indicators) return null;
    return indicators.signals
      .filter((s) => s.kind === "Buy" && s.pnl_pct != null)
      .reduce((sum, s) => sum + (s.pnl_pct as number), 0);
  }, [indicators]);

  return (
    <div className="app">
      <header className="app-header">
        <h1>
          <span className="dot">●</span> Koishi Stock Dashboard
        </h1>
        <StockSearch onSelect={handleAddStock} />
      </header>

      <div className="app-main">
        <aside className="sidebar">
          <StockList
            stocks={stocks}
            activeSymbol={activeSymbol}
            onSelect={setActiveSymbol}
            onRemove={handleRemoveStock}
          />
        </aside>

        <section className="chart-pane">
          {activeStock && !loadingChart && !chartError && candles.length > 0 && (
            <div className="chart-header">
              <div className="stock-label">
                <span className="symbol">{activeStock.symbol}</span>
                <span className="name">
                  {activeStock.name} · {activeStock.market}
                </span>
              </div>
              {totalClosedPnl !== null && (
                <div
                  className="pnl-badge"
                  style={{
                    color: totalClosedPnl >= 0 ? "#ef4444" : "#22c55e",
                  }}
                >
                  已平仓收益 {totalClosedPnl >= 0 ? "+" : ""}
                  {totalClosedPnl.toFixed(2)}%
                </div>
              )}
              {loadingMore && (
                <span style={{ fontSize: 12, color: "var(--text-muted)" }}>加载更多数据…</span>
              )}
            </div>
          )}

          {loadingChart && (
            <div className="placeholder">加载中…</div>
          )}

          {chartError && (
            <div className="placeholder" style={{ color: "var(--danger)" }}>
              加载失败: {chartError}
            </div>
          )}

          {!activeStock && !loadingChart && (
            <div className="placeholder">
              搜索并选择一支股票以查看 K 线图
            </div>
          )}

          {activeStock && !loadingChart && !chartError && candles.length > 0 && indicators && (
            <StockChart
              key={activeStock.symbol}
              candles={candles}
              bollinger={indicators.bollinger}
              keltner={indicators.keltner}
              macd={indicators.macd}
              regime={indicators.regime}
              kdj={indicators.kdj}
              signals={indicators.signals}
              label={activeStock.symbol}
              onReachLeftEdge={handleReachLeftEdge}
            />
          )}

          {activeStock && !loadingChart && !chartError && candles.length === 0 && (
            <div className="placeholder">暂无数据</div>
          )}
        </section>
      </div>
    </div>
  );
}