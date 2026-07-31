import type { IndicatorsResponse, StockInfo } from "./types";

const BASE = "/api";

export interface FactorToggle {
  quantile: boolean;
  reversal: boolean;
  divergence: boolean;
}

/** Fetch indicators. */
export async function fetchIndicators(
  symbol: string,
  days?: number,
  strategy: string = "default",
  forward: number = 5,
  toggle: FactorToggle = { quantile: true, reversal: true, divergence: true },
): Promise<IndicatorsResponse> {
  let url = `${BASE}/indicators?symbol=${encodeURIComponent(symbol)}`;
  if (days !== undefined) url += `&days=${days}`;
  url += `&strategy=${encodeURIComponent(strategy)}`;
  url += `&forward=${forward}`;
  url += `&quantile=${toggle.quantile}`;
  url += `&reversal=${toggle.reversal}`;
  url += `&divergence=${toggle.divergence}`;
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