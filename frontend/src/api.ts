import type { IndicatorsResponse, StockInfo } from "./types";

const BASE = "/api";

/** Fetch candles + Bollinger Bands + KDJ in one call. */
export async function fetchIndicators(symbol: string, days?: number): Promise<IndicatorsResponse> {
  let url = `${BASE}/indicators?symbol=${encodeURIComponent(symbol)}`;
  if (days !== undefined) url += `&days=${days}`;
  const res = await fetch(url);
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error((body as { error?: string }).error ?? `HTTP ${res.status}`);
  }
  return res.json();
}

export async function searchStocks(keyword: string): Promise<StockInfo[]> {
  const url = `${BASE}/search?keyword=${encodeURIComponent(keyword)}`;
  const res = await fetch(url);
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error((body as { error?: string }).error ?? `HTTP ${res.status}`);
  }
  return res.json();
}