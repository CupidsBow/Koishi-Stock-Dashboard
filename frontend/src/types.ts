export interface Candle {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export interface StockInfo {
  symbol: string;
  name: string;
  market: string;
}

export interface StockItem extends StockInfo {
  addedAt: number;
}

export interface BollingerPoint {
  time: number;
  upper: number;
  middle: number;
  lower: number;
}

export interface KeltnerPoint {
  time: number;
  upper: number;
  middle: number;
  lower: number;
}

export interface MacdPoint {
  time: number;
  dif: number;
  dea: number;
  bar: number;
}

export interface KdjPoint {
  time: number;
  k: number;
  d: number;
  j: number;
}

export interface AdxPoint {
  time: number;
  adx: number;
  plus_di: number;
  minus_di: number;
}

export interface Signal {
  time: number;
  kind: "Buy" | "Sell";
  price: number;
  reason: string;
}

export interface IndicatorsResponse {
  candles: Candle[];
  bollinger: (BollingerPoint | null)[];
  keltner: (KeltnerPoint | null)[];
  macd: (MacdPoint | null)[];
  kdj: (KdjPoint | null)[];
  adx: (AdxPoint | null)[];
  rsi: (number | null)[];
  regime: string;
  signals: Signal[];
}