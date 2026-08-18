import { useEffect, useRef, useState } from "react";
import {
  createChart,
  CandlestickSeries,
  HistogramSeries,
  LineSeries,
  type IChartApi,
  type ISeriesApi,
  type CandlestickData,
  type Time,
  ColorType,
} from "lightweight-charts";
import type { DrawingUtils } from "lightweight-charts";
import type {
  IPrimitivePaneRenderer,
  IPrimitivePaneView,
  ISeriesPrimitive,
  SeriesAttachedParameter,
} from "lightweight-charts";
import type { CanvasRenderingTarget2D, MediaCoordinatesRenderingScope } from "fancy-canvas";
import type { ActionPoint, Candle, BollingerPoint, KeltnerPoint, MacdPoint, KdjPoint, Signal } from "../types";

interface Props {
  candles: Candle[];
  bollinger: (BollingerPoint | null)[];
  keltner: (KeltnerPoint | null)[];
  macd: (MacdPoint | null)[];
  kdj: (KdjPoint | null)[];
  signals: Signal[];
  turtleActions?: ActionPoint[];
  regime?: string;
  label?: string;
  onReachLeftEdge?: () => void;
}

function toCandlestickData(c: Candle): CandlestickData {
  return { time: c.time as Time, open: c.open, high: c.high, low: c.low, close: c.close };
}

function toLine<T>(data: (T | null)[], map: (p: T) => { time: Time; value: number }) {
  return data.filter((p): p is T => p !== null).map(map);
}

// ── Pane view renderer that draws text labels for BB squeeze signals ─────

class SignalLabelRenderer implements IPrimitivePaneRenderer {
  constructor(
    private readonly _x: number,
    private readonly _y: number,
    _text: string,
    private readonly _color: string,
    private readonly _isBuy: boolean,
  ) {}

  draw(target: CanvasRenderingTarget2D, _utils?: DrawingUtils): void {
    target.useMediaCoordinateSpace((scope: MediaCoordinatesRenderingScope) => {
      const ctx = scope.context;
      ctx.save();

      const isBuy = this._isBuy;
      const color = this._color;
      const labelX = this._x;
      const labelY = this._y;

      // ── Compact dot: just a colored circle with "B" or "S" ──
      const r = 7;

      ctx.beginPath();
      ctx.arc(labelX, isBuy ? labelY - 2 : labelY + 2, r, 0, Math.PI * 2);
      ctx.fillStyle = color;
      ctx.fill();

      ctx.fillStyle = "#ffffff";
      ctx.font = "bold 10px sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(isBuy ? "B" : "S", labelX, isBuy ? labelY - 2 : labelY + 2);

      ctx.restore();
    });
  }

  }

// ── Pane view wrapper ─────────────────────────────────────────────────────

class SignalLabelPaneView implements IPrimitivePaneView {
  constructor(private readonly _renderer: SignalLabelRenderer) {}

  renderer(): IPrimitivePaneRenderer | null {
    return this._renderer;
  }
}

// ── Series primitive for BB squeeze signal labels ─────────────────────────

class BBSignalPrimitive implements ISeriesPrimitive<Time> {
  private _chart: IChartApi | null = null;
  private _series: ISeriesApi<"Candlestick", Time> | null = null;
  private _paneViews: readonly SignalLabelPaneView[] = [];
  private _data: { candles: Candle[]; signals: Signal[] } = { candles: [], signals: [] };
  private _requestUpdate: (() => void) | null = null;

  updateData(candles: Candle[], signals: Signal[]): void {
    this._data = { candles, signals };
    this._paneViews = this._buildViews();
    this._requestUpdate?.();
  }

  attached(param: SeriesAttachedParameter<Time>): void {
    this._chart = param.chart as IChartApi;
    this._series = param.series as ISeriesApi<"Candlestick", Time>;
    this._requestUpdate = param.requestUpdate;
  }

  detached(): void {
    this._chart = null;
    this._series = null;
    this._requestUpdate = null;
  }

  updateAllViews(): void {
    this._paneViews = this._buildViews();
  }

  paneViews(): readonly IPrimitivePaneView[] {
    return this._paneViews;
  }

  private _buildViews(): readonly SignalLabelPaneView[] {
    const { candles, signals } = this._data;
    const chart = this._chart;
    const series = this._series;
    if (!chart || !series || !candles.length) return [];

    const tradeSignals = signals.filter((s) => s.kind === "Buy" || s.kind === "Sell");
    if (!tradeSignals.length) return [];

    const candleByTime = new Map<number, Candle>();
    for (const c of candles) {
      candleByTime.set(c.time, c);
    }

    const timeScale = chart.timeScale();
    const views: SignalLabelPaneView[] = [];

    for (const s of tradeSignals) {
      const candle = candleByTime.get(s.time);
      if (!candle) continue;

      const x = timeScale.timeToCoordinate(s.time as Time);
      if (x === null) continue;

      const priceY = series.priceToCoordinate(
        s.kind === "Buy" ? candle.low : candle.high,
      );
      if (priceY === null) continue;

      const offsetY = s.kind === "Buy" ? 28 : -28;
      const color = s.kind === "Buy" ? "#dc2626" : "#16a34a";

      views.push(new SignalLabelPaneView(
        new SignalLabelRenderer(x, priceY + offsetY, s.reason, color, s.kind === "Buy"),
      ));
    }

    return views;
  }

  /** Find the signal closest to a chart time position. */
  getSignalsAt(
    time: number,
  ): Signal[] {
    const { signals } = this._data;
    return signals.filter(
      (s) => s.time === time && (s.kind === "Buy" || s.kind === "Sell")
    );
  }
}

// ── Turtle action label renderer ─────────────────────────────────────────

class TurtleActionRenderer implements IPrimitivePaneRenderer {
  constructor(
    private readonly _x: number,
    private readonly _y: number,
    private readonly _text: string,
    private readonly _color: string,
    private readonly _isExit: boolean,
  ) {}

  draw(target: CanvasRenderingTarget2D, _utils?: DrawingUtils): void {
    target.useMediaCoordinateSpace((scope: MediaCoordinatesRenderingScope) => {
      const ctx = scope.context;
      ctx.save();

      const r = 8;
      const x = this._x;
      const y = this._y;
      const color = this._color;

      if (this._isExit) {
        // Exit → rectangle "平"
        ctx.fillStyle = color;
        ctx.fillRect(x - r, y - r, r * 2, r * 2);
        ctx.fillStyle = "#ffffff";
        ctx.font = "bold 9px sans-serif";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillText("平", x, y);
      } else {
        // Entry/Add → diamond/circle
        ctx.beginPath();
        ctx.arc(x, y, r, 0, Math.PI * 2);
        ctx.fillStyle = color;
        ctx.fill();

        ctx.fillStyle = "#ffffff";
        ctx.font = "bold 9px sans-serif";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillText(this._text, x, y);
      }

      ctx.restore();
    });
  }
}

class TurtleActionPaneView implements IPrimitivePaneView {
  constructor(private readonly _renderer: TurtleActionRenderer) {}

  renderer(): IPrimitivePaneRenderer | null {
    return this._renderer;
  }
}

class TurtleActionPrimitive implements ISeriesPrimitive<Time> {
  private _chart: IChartApi | null = null;
  private _series: ISeriesApi<"Candlestick", Time> | null = null;
  private _paneViews: readonly TurtleActionPaneView[] = [];
  private _data: { candles: Candle[]; actions: ActionPoint[] } = { candles: [], actions: [] };
  private _requestUpdate: (() => void) | null = null;

  updateData(candles: Candle[], actions: ActionPoint[]): void {
    this._data = { candles, actions };
    this._paneViews = this._buildViews();
    this._requestUpdate?.();
  }

  attached(param: SeriesAttachedParameter<Time>): void {
    this._chart = param.chart as IChartApi;
    this._series = param.series as ISeriesApi<"Candlestick", Time>;
    this._requestUpdate = param.requestUpdate;
  }

  detached(): void {
    this._chart = null;
    this._series = null;
    this._requestUpdate = null;
  }

  updateAllViews(): void {
    this._paneViews = this._buildViews();
  }

  paneViews(): readonly IPrimitivePaneView[] {
    return this._paneViews;
  }

  private _buildViews(): readonly TurtleActionPaneView[] {
    const { candles, actions } = this._data;
    const chart = this._chart;
    const series = this._series;
    if (!chart || !series || !candles.length || !actions.length) return [];

    const candleByTime = new Map<number, Candle>();
    for (const c of candles) {
      candleByTime.set(c.time, c);
    }

    const timeScale = chart.timeScale();
    const views: TurtleActionPaneView[] = [];

    for (const a of actions) {
      const candle = candleByTime.get(a.time);
      if (!candle) continue;

      const x = timeScale.timeToCoordinate(a.time as Time);
      if (x === null) continue;

      const priceY = series.priceToCoordinate(
        a.action === "Exit" ? candle.high : candle.low,
      );
      if (priceY === null) continue;

      let text: string;
      let color: string;
      let offsetY: number;

      switch (a.action) {
        case "Entry":
          text = "建";
          color = "#dc2626";
          offsetY = a.action === "Entry" ? -40 : -25;
          break;
        case "Add":
          text = "+";
          color = "#ef4444";
          offsetY = -25;
          break;
        case "Exit":
          text = "平";
          color = "#16a34a";
          offsetY = a.action === "Exit" ? 40 : 28;
          break;
      }

      views.push(new TurtleActionPaneView(
        new TurtleActionRenderer(x, priceY + offsetY, text, color, a.action === "Exit"),
      ));
    }

    return views;
  }

  /** Find turtle actions at a given chart time. */
  getActionsAt(time: number): ActionPoint[] {
    return this._data.actions.filter((a) => a.time === time);
  }
}

// ── Component ─────────────────────────────────────────────────────────────

export default function StockChart({
  candles,
  bollinger,
  keltner,
  macd,
  kdj,
  signals,
  turtleActions,
  regime: _regime,
  label,
  onReachLeftEdge,
}: Props) {
  void (label as unknown);

  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const candleSeriesRef = useRef<ISeriesApi<"Candlestick"> | null>(null);
  const bbSignalPrimitiveRef = useRef<BBSignalPrimitive | null>(null);
  const turtlePrimitiveRef = useRef<TurtleActionPrimitive | null>(null);
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

  const signalsRef = useRef<Signal[]>(signals);
  signalsRef.current = signals;
  const turtleActionsRef = useRef<ActionPoint[]>(turtleActions || []);
  turtleActionsRef.current = turtleActions || [];
  const candlesRef = useRef<Candle[]>(candles);
  candlesRef.current = candles;
  const bbRef = useRef(bollinger);
  bbRef.current = bollinger;
  const knRef = useRef(keltner);
  knRef.current = keltner;
  const macdRef = useRef(macd);
  macdRef.current = macd;
  const kdjRef = useRef(kdj);
  kdjRef.current = kdj;
  const onReachLeftEdgeRef = useRef(onReachLeftEdge);
  const [hoveredSignals, setHoveredSignals] = useState<Signal[]>([]);
  const [hoveredActions, setHoveredActions] = useState<ActionPoint[]>([]);
  onReachLeftEdgeRef.current = onReachLeftEdge;
  const firedAtCount = useRef(0);

  useEffect(() => {
    if (!containerRef.current) return;

    const isDark =
      typeof window !== "undefined" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches;

    const chart = createChart(containerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: "transparent" },
        textColor: isDark ? "#d1d5db" : "#4b5563",
      },
      grid: {
        vertLines: { color: isDark ? "#2e3040" : "#e5e7eb" },
        horzLines: { color: isDark ? "#2e3040" : "#e5e7eb" },
      },
      crosshair: { mode: 0 },
      rightPriceScale: { borderColor: isDark ? "#2e3040" : "#e5e7eb" },
      timeScale: {
        borderColor: isDark ? "#2e3040" : "#e5e7eb",
        timeVisible: true,
        secondsVisible: false,
      },
      localization: {
        dateFormat: "yyyy-MM-dd",
      },
      width: containerRef.current.clientWidth,
      height: containerRef.current.clientHeight,
    });

    const candleSeries = chart.addSeries(CandlestickSeries, {
      upColor: "#ef4444",
      downColor: "#22c55e",
      borderUpColor: "#ef4444",
      borderDownColor: "#22c55e",
      wickUpColor: "#ef4444",
      wickDownColor: "#22c55e",
    });
    candleSeries.priceScale().applyOptions({ scaleMargins: { top: 0.05, bottom: 0.05 } });

    // Attach custom primitive for BB signal labels (replaces markers)
    const bbSignalPrimitive = new BBSignalPrimitive();
    candleSeries.attachPrimitive(bbSignalPrimitive);
    bbSignalPrimitiveRef.current = bbSignalPrimitive;

    // Attach turtle action primitive
    const turtlePrimitive = new TurtleActionPrimitive();
    candleSeries.attachPrimitive(turtlePrimitive);
    turtlePrimitiveRef.current = turtlePrimitive;

    const bbUpper = chart.addSeries(LineSeries, {
      color: "rgba(20,184,166,0.35)",
      lineWidth: 1,
    });
    const bbLower = chart.addSeries(LineSeries, {
      color: "rgba(20,184,166,0.35)",
      lineWidth: 1,
    });
    const knUpper = chart.addSeries(LineSeries, {
      color: "rgba(99,102,241,0.45)",
      lineWidth: 1,
    });
    const knMiddle = chart.addSeries(LineSeries, {
      color: "rgba(99,102,241,0.85)",
      lineWidth: 2,
    });
    const knLower = chart.addSeries(LineSeries, {
      color: "rgba(99,102,241,0.45)",
      lineWidth: 1,
    });

    const kdjK = chart.addSeries(LineSeries, { color: "#fbbf24", lineWidth: 1 }, 1);
    const kdjD = chart.addSeries(LineSeries, { color: "#f97316", lineWidth: 1 }, 1);
    const kdjJ = chart.addSeries(LineSeries, { color: "#a855f7", lineWidth: 1 }, 1);
    kdjK.priceScale().applyOptions({ scaleMargins: { top: 0.05, bottom: 0.05 } });

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

    chartRef.current = chart;
    candleSeriesRef.current = candleSeries;
    bbUpperRef.current = bbUpper;
    bbLowerRef.current = bbLower;
    knUpperRef.current = knUpper;
    knMiddleRef.current = knMiddle;
    knLowerRef.current = knLower;
    macdDifRef.current = macdDif;
    macdDeaRef.current = macdDea;
    macdBarRef.current = macdBar;
    kdjKRef.current = kdjK;
    kdjDRef.current = kdjD;
    kdjJRef.current = kdjJ;

    const observer = new ResizeObserver(() => {
      if (containerRef.current && chartRef.current) {
        chartRef.current.applyOptions({
          width: containerRef.current.clientWidth,
          height: containerRef.current.clientHeight,
        });
      }
    });
    observer.observe(containerRef.current);

    return () => {
      observer.disconnect();
      chart.remove();
      chartRef.current = null;
      candleSeriesRef.current = null;
      bbSignalPrimitiveRef.current = null;
      turtlePrimitiveRef.current = null;
      bbUpperRef.current = null;
      bbLowerRef.current = null;
      knUpperRef.current = null;
      knMiddleRef.current = null;
      knLowerRef.current = null;
      macdDifRef.current = null;
      macdDeaRef.current = null;
      macdBarRef.current = null;
      kdjKRef.current = null;
      kdjDRef.current = null;
      kdjJRef.current = null;
    };
  }, []);

  useEffect(() => {
    firedAtCount.current = 0;
  }, [candles]);

  // Update series data + BB signal labels
  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) return;
    const cs = candleSeriesRef.current;
    if (!cs) return;

    // Build KDJ/MACD cross-confirmed signal lookup for candle tinting
    const signalMap = new Map<number, "Buy" | "Sell">();
    for (const s of signalsRef.current) {
      if (s.reason.startsWith("买(") || s.reason.startsWith("卖(")) {
        signalMap.set(s.time, s.kind);
      }
    }

    // Tinted candle colors for KDJ/MACD cross-confirmed signal bars
    const BUY_COLOR = "#dc2626"; // deeper red for buy-signal bars
    const SELL_COLOR = "#16a34a"; // deeper green for sell-signal bars

    cs.setData(
      candlesRef.current.map((c) => {
        const base = toCandlestickData(c);
        const sigKind = signalMap.get(c.time);
        if (sigKind) {
          const tint = sigKind === "Buy" ? BUY_COLOR : SELL_COLOR;
          base.color = tint;
          base.borderColor = tint;
          base.wickColor = tint;
        }
        return base;
      }),
    );

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
      mc
        .filter((p): p is MacdPoint => p !== null)
        .map((p) => ({
          time: p.time as Time,
          value: p.bar,
          color: p.bar >= 0 ? "#ef444480" : "#22c55e80",
        })),
    );
    const kd = kdjRef.current;
    kdjKRef.current?.setData(toLine(kd, (p) => ({ time: p.time as Time, value: p.k })));
    kdjDRef.current?.setData(toLine(kd, (p) => ({ time: p.time as Time, value: p.d })));
    kdjJRef.current?.setData(toLine(kd, (p) => ({ time: p.time as Time, value: p.j })));

    // Update custom primitive with latest signal data
    bbSignalPrimitiveRef.current?.updateData(candlesRef.current, signalsRef.current);
    turtlePrimitiveRef.current?.updateData(candlesRef.current, turtleActionsRef.current);
    // Trigger chart redraw so primitive picks up new data
    chart.timeScale().applyOptions({ fixRightEdge: true });

    const ts = chart.timeScale();
    ts.applyOptions({ fixRightEdge: true });
    const data = candlesRef.current;
    if (data.length > 180) {
      const cr = ts.getVisibleLogicalRange();
      if (!cr || cr.to >= data.length - 10) {
        ts.setVisibleLogicalRange({ from: data.length - 180, to: data.length - 1 });
      }
    } else {
      ts.fitContent();
    }
  }, [candles, bollinger, keltner, macd, kdj, signals, turtleActions]);

  return (
    <div
      style={{
        position: "relative",
        flex: 1,
        display: "flex",
        flexDirection: "column",
        minHeight: 0,
      }}
      onMouseMove={(e) => {
        if (!chartRef.current || !candleSeriesRef.current) return;
        const rect = e.currentTarget.getBoundingClientRect();
        const cx = e.clientX - rect.left;
        const t = chartRef.current.timeScale().coordinateToTime(cx) as number;
        if (t) {
          const rt = candlesRef.current.reduce((prev, curr) =>
            Math.abs(curr.time - t) < Math.abs(prev.time - t) ? curr : prev
          ).time;
          const sig = bbSignalPrimitiveRef.current?.getSignalsAt(rt);
          setHoveredSignals(sig ?? []);
          const acts = turtlePrimitiveRef.current?.getActionsAt(rt);
          setHoveredActions(acts ?? []);
        } else {
          setHoveredSignals([]);
          setHoveredActions([]);
        }
      }}
      onMouseLeave={() => { setHoveredSignals([]); setHoveredActions([]); }}
    >
      <div
        style={{
          position: "absolute",
          inset: 0,
          zIndex: 0,
          pointerEvents: "none",
          borderRadius: 8,
          overflow: "hidden",
        }}
      >
        <div
          style={{
            height: "45%",
            background: "var(--chart-pane0)",
            borderBottom: "1px dashed var(--chart-divider)",
          }}
        />
        <div
          style={{
            height: "27%",
            background: "var(--chart-pane1)",
            borderBottom: "1px dashed var(--chart-divider)",
          }}
        />
        <div style={{ height: "28%", background: "var(--chart-pane2)" }} />
      </div>

      <div
        style={{
          position: "absolute",
          top: "8%",
          right: 6,
          zIndex: 2,
          fontSize: 11,
          color: "var(--text-muted)",
          pointerEvents: "none",
          writingMode: "vertical-rl",
        }}
      >
        价格/元
      </div>
      <div
        style={{
          position: "absolute",
          top: "53%",
          right: 6,
          zIndex: 2,
          fontSize: 11,
          color: "var(--text-muted)",
          pointerEvents: "none",
          writingMode: "vertical-rl",
        }}
      >
        KDJ/%
      </div>
      <div
        style={{
          position: "absolute",
          top: "78%",
          right: 6,
          zIndex: 2,
          fontSize: 11,
          color: "var(--text-muted)",
          pointerEvents: "none",
          writingMode: "vertical-rl",
        }}
      >
        MACD
      </div>

      <div ref={containerRef} className="chart-container" />

      {/* Hover tooltip */}
      {hoveredSignals.length > 0 && (
        <div
          style={{
            position: "absolute",
            left: "50%",
            bottom: 8,
            transform: "translateX(-50%)",
            display: "flex",
            flexDirection: "column",
            gap: 4,
            zIndex: 10,
            pointerEvents: "none",
            maxWidth: "90%",
          }}
        >
          {hoveredSignals.map((s: Signal, i: number) => (
            <div
              key={`sig-${i}`}
              style={{
                background: s.kind === "Buy" ? "#dc2626" : "#16a34a",
                color: "#fff",
                padding: "5px 14px",
                borderRadius: 6,
                fontSize: 12,
                fontWeight: 600,
                textAlign: "center",
                whiteSpace: "nowrap",
              }}
            >
              {s.kind === "Buy" ? "B" : "S"} {s.reason}
            </div>
          ))}
          {hoveredActions.map((a: ActionPoint, i: number) => (
            <div
              key={`act-${i}`}
              style={{
                background: a.action === "Exit" ? "#16a34a" : "#dc2626",
                color: "#fff",
                padding: "5px 14px",
                borderRadius: 6,
                fontSize: 12,
                fontWeight: 600,
                textAlign: "center",
                whiteSpace: "nowrap",
              }}
            >
              {a.action === "Entry" ? "建" : a.action === "Add" ? "+" : "平"} {a.reason}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}