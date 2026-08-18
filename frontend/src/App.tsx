import { useState, useCallback, useEffect, useRef, useMemo } from "react";
import { fetchIndicators, type FactorToggle, type TurtleParams, DEFAULT_TURTLE_PARAMS } from "./api";
import type { ActionPoint, Candle, StockItem, StockInfo, IndicatorsResponse, Signal } from "./types";
import StockSearch from "./components/StockSearch";
import StockList from "./components/StockList";
import StockChart from "./components/StockChart";
import "./App.css";

const STORAGE_KEY = "stock-dashboard:watchlist";
const MODE_KEY = "stock-dashboard:strategy";
const TOGGLE_KEY = "stock-dashboard:toggles";
const TURTLE_PARAMS_KEY = "stock-dashboard:turtle-params";
const INITIAL_DAYS = 400;
const LOAD_MORE_DAYS = [800, 1600, 3200, 10000];

type StrategyMode = "cta" | "alpha" | "turtle";

function loadMode(): StrategyMode {
  try {
    const v = localStorage.getItem(MODE_KEY);
    if (v === "turtle") return "turtle";
    return v === "cta" ? "cta" : "alpha";
  } catch {
    return "alpha";
  }
}

function loadTurtleParams(): TurtleParams {
  try {
    const raw = localStorage.getItem(TURTLE_PARAMS_KEY);
    if (!raw) return { ...DEFAULT_TURTLE_PARAMS };
    const parsed = JSON.parse(raw);
    return {
      turtle_entry: parsed.turtle_entry ?? DEFAULT_TURTLE_PARAMS.turtle_entry,
      turtle_add: parsed.turtle_add ?? DEFAULT_TURTLE_PARAMS.turtle_add,
      turtle_stop: parsed.turtle_stop ?? DEFAULT_TURTLE_PARAMS.turtle_stop,
      turtle_units: parsed.turtle_units ?? DEFAULT_TURTLE_PARAMS.turtle_units,
    };
  } catch {
    return { ...DEFAULT_TURTLE_PARAMS };
  }
}

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

const DEFAULT_TOGGLES: FactorToggle = { quantile: true, reversal: true, divergence: true };

function loadToggles(): FactorToggle {
  try {
    const raw = localStorage.getItem(TOGGLE_KEY);
    if (!raw) return { ...DEFAULT_TOGGLES };
    const parsed = JSON.parse(raw);
    return {
      quantile: parsed.quantile ?? true,
      reversal: parsed.reversal ?? true,
      divergence: parsed.divergence ?? true,
    };
  } catch {
    return { ...DEFAULT_TOGGLES };
  }
}

/** Merge older (more days) into newer (already on screen). */
function mergeIndicators(older: IndicatorsResponse, newer: IndicatorsResponse): IndicatorsResponse {
  const newerTimes = new Set(newer.candles.map((c) => c.time));

  const extraIdx: number[] = [];
  const overlapIdx: number[] = [];
  for (let i = 0; i < older.candles.length; i++) {
    if (!newerTimes.has(older.candles[i].time)) {
      extraIdx.push(i);
    } else {
      overlapIdx.push(i);
    }
  }

  const candles = [...extraIdx.map((i) => older.candles[i]), ...newer.candles];

  const pickOlder = <T,>(arr: (T | null)[]) => [
    ...extraIdx.map((i) => arr[i]),
    ...overlapIdx.map((i) => arr[i]),
  ];

  const dedupSignals = (old: Signal[], newerList: Signal[]) => [
    ...old.filter((s) => !newerTimes.has(s.time)),
    ...newerList,
  ];

  const dedupActions = (old: ActionPoint[], newerList: ActionPoint[]) => [
    ...old.filter((a) => !newerTimes.has(a.time)),
    ...newerList,
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
    signals:  dedupSignals(older.signals, newer.signals),
    factor_evals: older.factor_evals?.length ? older.factor_evals : newer.factor_evals,
    factor_scores: older.factor_scores?.length ? pickOlder(older.factor_scores) : (newer.factor_scores || []),
    signals_v2: dedupSignals(older.signals_v2 || [], newer.signals_v2 || []),
    turtle_actions: dedupActions(older.turtle_actions || [], newer.turtle_actions || []),
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
  const [strategyMode, setStrategyMode] = useState<StrategyMode>(loadMode);
  const [switchingMode, setSwitchingMode] = useState(false);
  const [signalToggles, setSignalToggles] = useState<FactorToggle>(loadToggles);
  const [turtleParams, setTurtleParams] = useState<TurtleParams>(loadTurtleParams);

  const loadStepRef = useRef(0);
  const loadingMoreRef = useRef(false);

  useEffect(() => { saveStocks(stocks); }, [stocks]);
  useEffect(() => { localStorage.setItem(MODE_KEY, strategyMode); }, [strategyMode]);
  useEffect(() => { localStorage.setItem(TOGGLE_KEY, JSON.stringify(signalToggles)); }, [signalToggles]);
  useEffect(() => { localStorage.setItem(TURTLE_PARAMS_KEY, JSON.stringify(turtleParams)); }, [turtleParams]);

  const selectedStrategy =
    strategyMode === "alpha" ? "factor"
    : strategyMode === "turtle" ? "turtle"
    : "default";

  // Initial load — full loading state when no data exists
  useEffect(() => {
    if (!activeSymbol) {
      setCandles([]);
      setIndicators(null);
      setChartError(null);
      loadStepRef.current = 0;
      return;
    }
    let cancelled = false;

    // Only show full-page loading when there is NO existing data at all
    if (!indicators) {
      setLoadingChart(true);
    } else {
      setSwitchingMode(true);
    }
    setChartError(null);
    loadStepRef.current = 0;

    fetchIndicators(activeSymbol, INITIAL_DAYS, selectedStrategy, 5, signalToggles, turtleParams)
      .then((data) => {
        if (!cancelled) {
          setCandles(data.candles);
          setIndicators(data);
          setLoadingChart(false);
          setSwitchingMode(false);
        }
      })
      .catch((e) => {
        if (!cancelled) {
          setChartError(e instanceof Error ? e.message : "Failed to load data");
          setCandles([]);
          setIndicators(null);
          setLoadingChart(false);
          setSwitchingMode(false);
        }
      });
    return () => { cancelled = true; };
  }, [activeSymbol, selectedStrategy]);

  
  // Refresh when toggles change (preserve lazy-load depth)
  useEffect(() => {
    if (!activeSymbol || !candles.length) return;
    let cancelled = false;
    setSwitchingMode(true);
    fetchIndicators(activeSymbol, candles.length, selectedStrategy, 5, signalToggles, turtleParams)
      .then((data) => {
        if (!cancelled) {
          setIndicators(data);
          setSwitchingMode(false);
        }
      })
      .catch(() => { if (!cancelled) setSwitchingMode(false); });
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signalToggles]);

  // Refresh when turtle params change (preserve lazy-load depth)
  useEffect(() => {
    if (!activeSymbol || !candles.length || strategyMode !== "turtle") return;
    let cancelled = false;
    setSwitchingMode(true);
    fetchIndicators(activeSymbol, candles.length, selectedStrategy, 5, signalToggles, turtleParams)
      .then((data) => {
        if (!cancelled) {
          setIndicators(data);
          setSwitchingMode(false);
        }
      })
      .catch(() => { if (!cancelled) setSwitchingMode(false); });
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [turtleParams]);
  
  // Lazy load more history
  const handleReachLeftEdge = useCallback(async () => {
    if (!activeSymbol || loadingMoreRef.current) return;
    const nextStep = loadStepRef.current;
    if (nextStep >= LOAD_MORE_DAYS.length) return;

    const days = LOAD_MORE_DAYS[nextStep];
    loadingMoreRef.current = true;
    setLoadingMore(true);

    try {
      const data = await fetchIndicators(activeSymbol, days, selectedStrategy, 5, signalToggles, turtleParams);
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
      // silently fail
    } finally {
      loadingMoreRef.current = false;
      setLoadingMore(false);
    }
  }, [activeSymbol, selectedStrategy, signalToggles, turtleParams]);

  const handleAddStock = useCallback(
    (stock: StockInfo) => {
      setStocks((prev) => {
        if (prev.some((s) => s.symbol === stock.symbol)) return prev;
        return [...prev, { ...stock, addedAt: Date.now() }];
      });
      setActiveSymbol(stock.symbol);
    },
    []
  );

  const handleRemoveStock = useCallback((symbol: string) => {
    setStocks((prev) => prev.filter((s) => s.symbol !== symbol));
    setActiveSymbol((current) => (current === symbol ? null : current));
  }, []);

  const handleReorderStock = useCallback(
    (fromIndex: number, toIndex: number) => {
      setStocks((prev) => {
        const next = [...prev];
        const [moved] = next.splice(fromIndex, 1);
        next.splice(toIndex, 0, moved);
        return next;
      });
    },
    []
  );

  const activeStock = stocks.find((s) => s.symbol === activeSymbol);

  // Current signals depend on strategy mode
  const displaySignals = useMemo(() => {
    if (!indicators) return [];
    return strategyMode === "alpha"
      ? indicators.signals_v2 || []
      : indicators.signals;
  }, [indicators, strategyMode]);

  // Turtle actions
  const displayActions = useMemo(() => {
    if (!indicators) return [];
    return strategyMode === "turtle" ? indicators.turtle_actions || [] : [];
  }, [indicators, strategyMode]);

  // ── PnL (same for both cta and alpha signals) ──
  const currentPnl = useMemo(() => {
    if (!indicators) return null;
    return displaySignals
      .filter((s) => s.kind === "Buy" && s.pnl_pct != null)
      .reduce((sum, s) => sum + (s.pnl_pct as number), 0);
  }, [indicators, displaySignals]);

  // ── Turtle stats ──────────────────────────────────────
  const turtleTrades = useMemo(() => {
    return displayActions.filter((a) => a.action === "Exit").length;
  }, [displayActions]);

  const turtleWinRate = useMemo(() => {
    const exits = displayActions.filter((a) => a.action === "Exit" && a.pnl_pct != null);
    if (!exits.length) return null;
    const winners = exits.filter((a) => a.pnl_pct! > 0).length;
    return winners / exits.length;
  }, [displayActions]);

  const turtleTotalPnl = useMemo(() => {
    return displayActions
      .filter((a) => a.action === "Exit" && a.pnl_pct != null)
      .reduce((sum, a) => sum + a.pnl_pct!, 0);
  }, [displayActions]);

  // ── Factor stats ──────────────────────────────────────
  const validFactorCount = useMemo(() => {
    if (!indicators?.factor_evals) return 0;
    return indicators.factor_evals.filter((e) => e.is_valid).length;
  }, [indicators]);

  const alphaScore = useMemo(() => {
    if (!indicators?.factor_scores) return null;
    const scores = indicators.factor_scores.filter((s) => s !== null);
    if (!scores.length) return null;
    return scores[scores.length - 1]!.total;
  }, [indicators]);

  const pnlLabel = strategyMode === "alpha" ? "因子收益"
    : strategyMode === "turtle" ? "规则收益"
    : "规则收益";

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
            onReorder={handleReorderStock}
          />
        </aside>

        <section className="chart-pane">
          {/* ---- first load (no data at all) ---- */}
          {loadingChart && (
            <div className="placeholder">加载中…</div>
          )}

          {/* ---- error ---- */}
          {!loadingChart && chartError && (
            <div className="placeholder" style={{ color: "var(--danger)" }}>
              加载失败: {chartError}
            </div>
          )}

          {/* ---- no stock selected ---- */}
          {!loadingChart && !activeStock && (
            <div className="placeholder">搜索并选择一支股票以查看 K 线图</div>
          )}

          {/* ---- no data ---- */}
          {!loadingChart && activeStock && !chartError && candles.length === 0 && (
            <div className="placeholder">暂无数据</div>
          )}

          {/* ---- chart with data (always visible during strategy switch) ---- */}
          {activeStock && !chartError && candles.length > 0 && indicators && (
            <>
              <div className="chart-header">
                <div className="header-left">
                  <div className="stock-label">
                    <span className="symbol">{activeStock.symbol}</span>
                    <span className="name">{activeStock.name} · {activeStock.market}</span>
                  </div>
                </div>

                <div className="header-center">
                  {/* Strategy toggle */}
                  <div className="strategy-switch-wrapper">
                    <div className="strategy-switch">
                      <span
                        className={`switch-option ${strategyMode === "cta" ? "active" : ""}`}
                        onClick={() => setStrategyMode("cta")}
                      >
                        CTA规则
                      </span>
                      <span
                        className={`switch-option ${strategyMode === "alpha" ? "active" : ""}`}
                        onClick={() => setStrategyMode("alpha")}
                      >
                        因子Alpha
                      </span>
                      <span
                        className={`switch-option ${strategyMode === "turtle" ? "active" : ""}`}
                        onClick={() => setStrategyMode("turtle")}
                      >
                        海龟仓位
                      </span>
                    </div>
                  </div>

                  {/* Signal type toggles (alpha mode only) */}
                  <div className={`signal-toggles ${strategyMode === "alpha" ? "visible" : "hidden"}`}>
                    <label className="toggle-label">
                      <input type="checkbox" checked={signalToggles.quantile}
                        onChange={(e) => setSignalToggles({ ...signalToggles, quantile: e.target.checked })} />
                      分位数
                    </label>
                    <label className="toggle-label">
                      <input type="checkbox" checked={signalToggles.reversal}
                        onChange={(e) => setSignalToggles({ ...signalToggles, reversal: e.target.checked })} />
                      转折
                    </label>
                    <label className="toggle-label">
                      <input type="checkbox" checked={signalToggles.divergence}
                        onChange={(e) => setSignalToggles({ ...signalToggles, divergence: e.target.checked })} />
                      背离
                    </label>
                  </div>

                  {/* Turtle params + stats block (turtle mode only) */}
                  <div className={`turtle-params ${strategyMode === "turtle" ? "visible" : "hidden"}`}>
                    <div className="turtle-params-row">
                      <label className="turtle-label">
                        <span className="turtle-label-head">建仓 <em>{turtleParams.turtle_entry.toFixed(1)}σ</em></span>
                        <input type="range" min={0.5} max={3.0} step={0.1}
                          value={turtleParams.turtle_entry}
                          onChange={(e) => setTurtleParams({ ...turtleParams, turtle_entry: +e.target.value })} />
                      </label>
                      <label className="turtle-label">
                        <span className="turtle-label-head">加仓 <em>{turtleParams.turtle_add.toFixed(1)}σ</em></span>
                        <input type="range" min={0.2} max={1.5} step={0.1}
                          value={turtleParams.turtle_add}
                          onChange={(e) => setTurtleParams({ ...turtleParams, turtle_add: +e.target.value })} />
                      </label>
                    </div>
                    <div className="turtle-params-row">
                      <label className="turtle-label">
                        <span className="turtle-label-head">止损 <em>{turtleParams.turtle_stop.toFixed(1)}σ</em></span>
                        <input type="range" min={1.0} max={4.0} step={0.1}
                          value={turtleParams.turtle_stop}
                          onChange={(e) => setTurtleParams({ ...turtleParams, turtle_stop: +e.target.value })} />
                      </label>
                      <label className="turtle-label">
                        <span className="turtle-label-head">层数 <em>{turtleParams.turtle_units}</em></span>
                        <input type="range" min={2} max={6} step={1}
                          value={turtleParams.turtle_units}
                          onChange={(e) => setTurtleParams({ ...turtleParams, turtle_units: +e.target.value })} />
                      </label>
                    </div>
                    {/* Inline stats summary — sits below the sliders, no overflow to header-right */}
                    <div className="turtle-stats-summary">
                      {turtleTrades > 0 ? (
                        <>
                          交易 <em>{turtleTrades}</em> 笔
                          {turtleWinRate !== null && (
                            <span> | 胜率 <em>{(turtleWinRate * 100).toFixed(0)}%</em></span>
                          )}
                          <span> | PnL <em style={{ color: turtleTotalPnl >= 0 ? "#ef4444" : "#22c55e" }}>
                            {turtleTotalPnl >= 0 ? "+" : ""}{turtleTotalPnl.toFixed(2)}%
                          </em></span>
                        </>
                      ) : (
                        <span className="turtle-stats-empty">暂无交易记录</span>
                      )}
                    </div>
                  </div>

                  {/* Loading indicator — fixed-width slot */}
                  <span className="loading-slot">
                    {switchingMode ? "加载新策略…" : ""}
                  </span>
                </div>

                <div className="header-right">
                  {/* Factor info — alpha mode only (turtle stats live in turtle-params block) */}
                  <div className={`info-slot ${strategyMode === "alpha" ? "visible" : "hidden"}`}>
                    {indicators.factor_evals && indicators.factor_evals.length > 0 && (
                      <div className="factor-badge">
                        因子 {validFactorCount}/{indicators.factor_evals?.length ?? 0} 有效
                        {alphaScore !== null && (
                          <span className="alpha-score" style={{
                            color: alphaScore >= 0 ? "#ef4444" : "#22c55e",
                          }}>
                            {" "}Alpha {alphaScore.toFixed(2)}
                          </span>
                        )}
                      </div>
                    )}
                  </div>

                  {/* PnL badge */}
                  {currentPnl !== null && (
                    <div
                      className="pnl-badge"
                      style={{ color: currentPnl >= 0 ? "#ef4444" : "#22c55e" }}
                    >
                      {pnlLabel} {currentPnl >= 0 ? "+" : ""}{currentPnl.toFixed(2)}%
                    </div>
                  )}

                </div>
                {/* Lazy-load — absolute overlay on chart container */}
                {loadingMore && (
                  <span style={{ position: "absolute", right: 16, top: 14, fontSize: 12, color: "var(--text-muted)", zIndex: 3 }}>
                    加载更多数据…
                  </span>
                )}
              </div>

              <StockChart
                key={activeSymbol}
                candles={candles}
                bollinger={indicators.bollinger}
                keltner={indicators.keltner}
                macd={indicators.macd}
                regime={indicators.regime}
                kdj={indicators.kdj}
                signals={displaySignals}
                turtleActions={displayActions}
                label={activeStock.symbol}
                onReachLeftEdge={handleReachLeftEdge}
              />
            </>
          )}
        </section>
      </div>
    </div>
  );
}