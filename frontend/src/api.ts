import type { IndicatorsResponse, StockInfo } from "./types";

const BASE = "/api";

export interface FactorToggle {
  quantile: boolean;
  reversal: boolean;
  divergence: boolean;
}

/** Turtle strategy parameters. */
export interface TurtleParams {
  turtle_entry: number;
  turtle_add: number;
  turtle_stop: number;
  turtle_units: number;
}

export const DEFAULT_TURTLE_PARAMS: TurtleParams = {
  turtle_entry: 1.5,
  turtle_add: 0.5,
  turtle_stop: 2.0,
  turtle_units: 4,
};

/** Fetch indicators. */
export async function fetchIndicators(
  symbol: string,
  days?: number,
  strategy: string = "default",
  forward: number = 5,
  toggle: FactorToggle = { quantile: true, reversal: true, divergence: true },
  turtleParams: TurtleParams = DEFAULT_TURTLE_PARAMS,
): Promise<IndicatorsResponse> {
  let url = `${BASE}/indicators?symbol=${encodeURIComponent(symbol)}`;
  if (days !== undefined) url += `&days=${days}`;
  url += `&strategy=${encodeURIComponent(strategy)}`;
  url += `&forward=${forward}`;
  url += `&quantile=${toggle.quantile}`;
  url += `&reversal=${toggle.reversal}`;
  url += `&divergence=${toggle.divergence}`;
  // Turtle params (only used when strategy=turtle)
  url += `&turtle_entry=${turtleParams.turtle_entry}`;
  url += `&turtle_add=${turtleParams.turtle_add}`;
  url += `&turtle_stop=${turtleParams.turtle_stop}`;
  url += `&turtle_units=${turtleParams.turtle_units}`;
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