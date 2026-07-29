import { useEffect, useRef, useState, useCallback } from "react";
import {
  createChart,
  CandlestickSeries,
  HistogramSeries,
  LineSeries,
  createSeriesMarkers,
  type IChartApi,
  type ISeriesApi,
  type ISeriesMarkersPluginApi,
  type CandlestickData,
  type Time,
  type MouseEventParams,
  ColorType,
} from "lightweight-charts";
import { fetchMinutes } from "../api";
import type { Candle, BollingerPoint, KeltnerPoint, MacdPoint, KdjPoint, Signal } from "../types";

interface Props {
  candles: Candle[];
  bollinger: (BollingerPoint | null)[];
  keltner: (KeltnerPoint | null)[];
  macd: (MacdPoint | null)[];
  kdj: (KdjPoint | null)[];
  signals: Signal[];
  label?: string;
  onReachLeftEdge?: () => void;
}

function toCandlestickData(c: Candle): CandlestickData {
  return { time: c.time as Time, open: c.open, high: c.high, low: c.low, close: c.close };
}

function toLine<T>(data: (T | null)[], map: (p: T) => { time: Time; value: number }) {
  return data.filter((p): p is T => p !== null).map(map);
}

function timestampToDate(ts: number): string {
  const d = new Date(ts * 1000);
  return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,"0")}-${String(d.getDate()).padStart(2,"0")}`;
}

export default function StockChart({ candles, bollinger, keltner, macd, kdj, signals, label, onReachLeftEdge }: Props) {
  const stockSymbol = label ?? "";

  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const candleSeriesRef = useRef<ISeriesApi<"Candlestick"> | null>(null);
  const markersRef = useRef<ISeriesMarkersPluginApi<Time> | null>(null);
  const bbUpperRef = useRef<ISeriesApi<"Line"> | null>(null);
  const bbLowerRef = useRef<ISeriesApi<"Line"> | null>(null);
  const knUpperRef = useRef<ISeriesApi<"Line"> | null>(null);
  const knMiddleRef = useRef<ISeriesApi<"Line"> | null>(null);
  const knLowerRef = useRef<ISeriesApi<"Line"> | null>(null);
  const macdDifRef = useRef<ISeriesApi<"Line"> | null>(null);
  const macdDeaRef = useRef<ISeriesApi<"Line"> | null>(null);
  const macdBarRef = useRef<ISeriesApi<"Histogram"> | null>(null);
  const kdjKRef = useRef<ISeriesApi<"Line"> | null>(null);
  const kdjDRef = useRef<ISeriesApi<"Line"> | null>(null);
  const kdjJRef = useRef<ISeriesApi<"Line"> | null>(null);

  const [view, setView] = useState<"daily" | "minute">("daily");
  const viewRef = useRef(view); viewRef.current = view;
  const [minuteCandles, setMinuteCandles] = useState<Candle[]>([]);
  const [minuteLoading, setMinuteLoading] = useState(false);
  const [minuteError, setMinuteError] = useState<string | null>(null);
  const [clickedDate, setClickedDate] = useState<string | null>(null);

  const candlesRef = useRef<Candle[]>(candles); candlesRef.current = candles;
  const symbolRef = useRef(stockSymbol); symbolRef.current = stockSymbol;
  const bbRef = useRef(bollinger); bbRef.current = bollinger;
  const knRef = useRef(keltner); knRef.current = keltner;
  const macdRef = useRef(macd); macdRef.current = macd;
  const kdjRef = useRef(kdj); kdjRef.current = kdj;
  const signalsRef = useRef<Signal[]>(signals); signalsRef.current = signals;
  const onReachLeftEdgeRef = useRef(onReachLeftEdge);
  onReachLeftEdgeRef.current = onReachLeftEdge;

  const firedAtCount = useRef(0);

  // ── Create chart ──────────────────────────────────────────────────
  useEffect(() => {
    if (!containerRef.current) return;

    const isDark = typeof window !== "undefined" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches;

    const chart = createChart(containerRef.current, {
      layout: { background: { type: ColorType.Solid, color: "transparent" }, textColor: isDark ? "#d1d5db" : "#4b5563" },
      grid: { vertLines: { color: isDark ? "#2e3040" : "#e5e7eb" }, horzLines: { color: isDark ? "#2e3040" : "#e5e7eb" } },
      crosshair: { mode: 0 },
      rightPriceScale: { borderColor: isDark ? "#2e3040" : "#e5e7eb" },
      timeScale: { borderColor: isDark ? "#2e3040" : "#e5e7eb", timeVisible: true, secondsVisible: false },
      width: containerRef.current.clientWidth,
      height: containerRef.current.clientHeight,
    });

    // Pane 0: Candlestick + BB + KC
    const candleSeries = chart.addSeries(CandlestickSeries, {
      upColor: "#ef4444", downColor: "#22c55e",
      borderUpColor: "#ef4444", borderDownColor: "#22c55e",
      wickUpColor: "#ef4444", wickDownColor: "#22c55e",
    });
    candleSeries.priceScale().applyOptions({ scaleMargins: { top: 0.05, bottom: 0.05 } });

    const bbUpper = chart.addSeries(LineSeries, { color: "rgba(20,184,166,0.35)", lineWidth: 1 });
    const bbLower = chart.addSeries(LineSeries, { color: "rgba(20,184,166,0.35)", lineWidth: 1 });
    const knUpper = chart.addSeries(LineSeries, { color: "rgba(99,102,241,0.45)", lineWidth: 1 });
    const knMiddle = chart.addSeries(LineSeries, { color: "rgba(99,102,241,0.85)", lineWidth: 2 });
    const knLower = chart.addSeries(LineSeries, { color: "rgba(99,102,241,0.45)", lineWidth: 1 });

    const markers = createSeriesMarkers(candleSeries, []);
    markersRef.current = markers;

    // Pane 1: KDJ
    const kdjK = chart.addSeries(LineSeries, { color: "#fbbf24", lineWidth: 1 }, 1);
    const kdjD = chart.addSeries(LineSeries, { color: "#f97316", lineWidth: 1 }, 1);
    const kdjJ = chart.addSeries(LineSeries, { color: "#a855f7", lineWidth: 1 }, 1);
    kdjK.priceScale().applyOptions({ scaleMargins: { top: 0.05, bottom: 0.05 } });

    // Pane 2: MACD
    const macdDif = chart.addSeries(LineSeries, { color: "#e2e8f0", lineWidth: 1 }, 2);
    const macdDea = chart.addSeries(LineSeries, { color: "#fbbf24", lineWidth: 1 }, 2);
    const macdBar = chart.addSeries(HistogramSeries, { priceFormat: { type: "volume", precision: 2, minMove: 0.01 } }, 2);
    macdDif.priceScale().applyOptions({ scaleMargins: { top: 0.05, bottom: 0.05 } });

    const handleVisibleRangeChange = () => {
      const range = chart.timeScale().getVisibleLogicalRange();
      if (!range) return;
      const count = candlesRef.current.length;
      if (range.from <= 2 && count > 0 && firedAtCount.current !== count) {
        firedAtCount.current = count;
        onReachLeftEdgeRef.current?.();
      }
    };
    chart.timeScale().subscribeVisibleLogicalRangeChange(handleVisibleRangeChange);

    const handleClick = (param: MouseEventParams) => {
      if (viewRef.current !== "daily") return;
      if (!param.time) return;
      const clicked = param.time as number;
      const dayCandle = candlesRef.current.find((c) => c.time === clicked);
      if (!dayCandle) return;
      const dateStr = timestampToDate(dayCandle.time);
      setClickedDate(dateStr);
      setView("minute"); setMinuteLoading(true); setMinuteError(null); setMinuteCandles([]);
      fetchMinutes(symbolRef.current, "5")
        .then((data) => { setMinuteCandles(data); setMinuteLoading(false); })
        .catch((e) => { setMinuteError(e instanceof Error ? e.message : "Failed"); setMinuteLoading(false); });
    };
    chart.subscribeClick(handleClick);

    chartRef.current = chart;
    candleSeriesRef.current = candleSeries;
    markersRef.current = markers;
    bbUpperRef.current = bbUpper; bbLowerRef.current = bbLower;
    knUpperRef.current = knUpper; knMiddleRef.current = knMiddle; knLowerRef.current = knLower;
    macdDifRef.current = macdDif; macdDeaRef.current = macdDea; macdBarRef.current = macdBar;
    kdjKRef.current = kdjK; kdjDRef.current = kdjD; kdjJRef.current = kdjJ;

    const observer = new ResizeObserver(() => {
      if (containerRef.current && chartRef.current) {
        chartRef.current.applyOptions({ width: containerRef.current.clientWidth, height: containerRef.current.clientHeight });
      }
    });
    observer.observe(containerRef.current);

    return () => {
      observer.disconnect(); chart.remove();
      chartRef.current = null; candleSeriesRef.current = null;
      markersRef.current = null;
      bbUpperRef.current = null; bbLowerRef.current = null;
      knUpperRef.current = null; knMiddleRef.current = null; knLowerRef.current = null;
      macdDifRef.current = null; macdDeaRef.current = null; macdBarRef.current = null;
      kdjKRef.current = null; kdjDRef.current = null; kdjJRef.current = null;
    };
  }, []);

  useEffect(() => { firedAtCount.current = 0; }, [candles]);

  // ── Update series data ──────────────────────────────────────────
  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) return;
    const cs = candleSeriesRef.current;
    if (!cs) return;

    const isDaily = view === "daily";
    const data = isDaily ? candlesRef.current : minuteCandles;

    cs.setData(data.map(toCandlestickData));

    if (isDaily) {
      const bb = bbRef.current;
      bbUpperRef.current?.setData(toLine(bb, (p) => ({ time: p.time as Time, value: p.upper })));
      bbLowerRef.current?.setData(toLine(bb, (p) => ({ time: p.time as Time, value: p.lower })));
      const kn = knRef.current;
      knUpperRef.current?.setData(toLine(kn, (p) => ({ time: p.time as Time, value: p.upper })));
      knMiddleRef.current?.setData(toLine(kn, (p) => ({ time: p.time as Time, value: p.middle })));
      knLowerRef.current?.setData(toLine(kn, (p) => ({ time: p.time as Time, value: p.lower })));
      const mc = macdRef.current;
      macdDifRef.current?.setData(toLine(mc, (p) => ({ time: p.time as Time, value: p.dif })));
      macdDeaRef.current?.setData(toLine(mc, (p) => ({ time: p.time as Time, value: p.dea })));
      macdBarRef.current?.setData(
        mc.filter((p): p is MacdPoint => p !== null).map((p) => ({
          time: p.time as Time,
          value: p.bar,
          color: p.bar >= 0 ? "#ef444480" : "#22c55e80",
        }))
      );
      const kd = kdjRef.current;
      kdjKRef.current?.setData(toLine(kd, (p) => ({ time: p.time as Time, value: p.k })));
      kdjDRef.current?.setData(toLine(kd, (p) => ({ time: p.time as Time, value: p.d })));
      kdjJRef.current?.setData(toLine(kd, (p) => ({ time: p.time as Time, value: p.j })));

      markersRef.current?.setMarkers(
        signalsRef.current.map((s) => ({
          time: s.time as Time,
          position: s.kind === "Buy" ? "belowBar" : "aboveBar",
          color: s.kind === "Buy" ? "#ff4444" : "#22c55e",
          shape: s.kind === "Buy" ? "arrowUp" : "arrowDown",
          text: s.reason,
          size: 2,
        }))
      );
    } else {
      bbUpperRef.current?.setData([]); bbLowerRef.current?.setData([]);
      knUpperRef.current?.setData([]); knMiddleRef.current?.setData([]); knLowerRef.current?.setData([]);
      macdDifRef.current?.setData([]); macdDeaRef.current?.setData([]); macdBarRef.current?.setData([]);
      kdjKRef.current?.setData([]); kdjDRef.current?.setData([]); kdjJRef.current?.setData([]);
      markersRef.current?.setMarkers([]);
    }

    const ts = chart.timeScale();
    ts.applyOptions({ fixRightEdge: true });
    if (isDaily && data.length > 180) {
      const cr = ts.getVisibleLogicalRange();
      if (!cr || cr.to >= data.length - 10) {
        ts.setVisibleLogicalRange({ from: data.length - 180, to: data.length - 1 });
      }
    } else {
      ts.fitContent();
    }
  }, [candles, bollinger, keltner, macd, kdj, view, minuteCandles]);

  const handleBack = useCallback(() => {
    setView("daily"); setMinuteCandles([]); setMinuteError(null); setClickedDate(null);
  }, []);

  return (
    <div style={{ position: "relative", flex: 1, display: "flex", flexDirection: "column", minHeight: 0 }}>
      <div style={{ position: "absolute", inset: 0, zIndex: 0, pointerEvents: "none", borderRadius: 8, overflow: "hidden" }}>
        <div style={{ height: "45%", background: "var(--chart-pane0)", borderBottom: "1px dashed var(--chart-divider)" }} />
        <div style={{ height: "27%", background: "var(--chart-pane1)", borderBottom: "1px dashed var(--chart-divider)" }} />
        <div style={{ height: "28%", background: "var(--chart-pane2)" }} />
      </div>

      <div style={{ position: "absolute", top: "8%",  right: 6, zIndex: 2, fontSize: 11, color: "var(--text-muted)", pointerEvents: "none", writingMode: "vertical-rl" }}>价格/元</div>
      <div style={{ position: "absolute", top: "53%", right: 6, zIndex: 2, fontSize: 11, color: "var(--text-muted)", pointerEvents: "none", writingMode: "vertical-rl" }}>KDJ/%</div>
      <div style={{ position: "absolute", top: "78%", right: 6, zIndex: 2, fontSize: 11, color: "var(--text-muted)", pointerEvents: "none", writingMode: "vertical-rl" }}>MACD</div>

      {view === "minute" && (
        <button onClick={handleBack} style={{ position: "absolute", top: 8, left: 12, zIndex: 10, background: "var(--bg-card)", color: "var(--text-h)", border: "1px solid var(--border)", borderRadius: 6, padding: "6px 14px", fontSize: 13, cursor: "pointer", fontWeight: 500, boxShadow: "var(--shadow)" }}>返回日线</button>
      )}
      {view === "minute" && clickedDate && (
        <div style={{ position: "absolute", top: 10, left: 0, right: 0, zIndex: 9, textAlign: "center", pointerEvents: "none" }}>
          <span style={{ background: "var(--bg-card)", color: "var(--text-h)", padding: "4px 12px", borderRadius: 6, fontSize: 13, fontWeight: 600, border: "1px solid var(--border)" }}>{stockSymbol}  {clickedDate}</span>
        </div>
      )}
      {view === "minute" && minuteLoading && (
        <div style={{ position: "absolute", inset: 0, zIndex: 5, display: "flex", alignItems: "center", justifyContent: "center", color: "var(--text-muted)", fontSize: 14 }}></div>
      )}
      {view === "minute" && minuteError && (
        <div style={{ position: "absolute", inset: 0, zIndex: 5, display: "flex", alignItems: "center", justifyContent: "center", color: "var(--danger)", fontSize: 14 }}>{minuteError}</div>
      )}

      <div ref={containerRef} className="chart-container" />
    </div>
  );
}