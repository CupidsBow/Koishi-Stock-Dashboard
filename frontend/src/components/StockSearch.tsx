import { useState, useRef, useEffect, useCallback } from "react";
import { searchStocks } from "../api";
import type { StockInfo } from "../types";

interface Props {
  /** Called when the user selects a stock from the search results. */
  onSelect: (stock: StockInfo) => void;
}

export default function StockSearch({ onSelect }: Props) {
  const [keyword, setKeyword] = useState("");
  const [results, setResults] = useState<StockInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState(false);

  const containerRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  // Close dropdown when clicking outside
  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, []);

  const doSearch = useCallback(async (kw: string) => {
    if (kw.trim().length === 0) {
      setResults([]);
      setOpen(false);
      setError(null);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const data = await searchStocks(kw.trim());
      setResults(data);
      setOpen(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Search failed");
      setResults([]);
      setOpen(true);
    } finally {
      setLoading(false);
    }
  }, []);

  const handleInput = (value: string) => {
    setKeyword(value);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => doSearch(value), 300);
  };

  const handleSelect = (stock: StockInfo) => {
    onSelect(stock);
    setKeyword("");
    setResults([]);
    setOpen(false);
  };

  return (
    <div className="stock-search" ref={containerRef}>
      <input
        type="text"
        placeholder="搜索股票代码或名称…"
        value={keyword}
        onChange={(e) => handleInput(e.target.value)}
        onFocus={() => {
          if (results.length > 0 || error) setOpen(true);
        }}
      />

      {open && (
        <div className="dropdown">
          {loading && <div className="loading">搜索中…</div>}
          {error && <div className="error">{error}</div>}
          {!loading &&
            !error &&
            results.length === 0 &&
            keyword.trim() && (
              <div className="no-results">未找到匹配的股票</div>
            )}
          {!loading &&
            !error &&
            results.map((s) => (
              <div
                key={s.symbol}
                className="dropdown-item"
                onClick={() => handleSelect(s)}
              >
                <span>
                  <span className="code">{s.symbol}</span>
                  <span className="market">{s.market}</span>
                </span>
                <span className="name">{s.name}</span>
              </div>
            ))}
        </div>
      )}
    </div>
  );
}