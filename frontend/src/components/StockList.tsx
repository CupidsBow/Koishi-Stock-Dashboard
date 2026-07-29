import type { StockItem } from "../types";

interface Props {
  /** All stocks the user has added. */
  stocks: StockItem[];
  /** The currently-selected symbol (or null). */
  activeSymbol: string | null;
  /** Called when the user clicks a stock to view it. */
  onSelect: (symbol: string) => void;
  /** Called when the user removes a stock. */
  onRemove: (symbol: string) => void;
}

export default function StockList({
  stocks,
  activeSymbol,
  onSelect,
  onRemove,
}: Props) {
  return (
    <div className="stock-list">
      <div className="list-header">
        <span>自选股票</span>
        {stocks.length > 0 && <span className="count">{stocks.length}</span>}
      </div>

      <div className="list-items">
        {stocks.length === 0 && (
          <div className="empty">
            暂无自选股票
            <br />
            使用上方搜索框添加
          </div>
        )}

        {stocks.map((s) => (
          <div
            key={s.symbol}
            className={`list-item${s.symbol === activeSymbol ? " active" : ""}`}
            onClick={() => onSelect(s.symbol)}
          >
            <div className="info">
              <div className="code">{s.symbol}</div>
              <div className="name">
                {s.name} · {s.market}
              </div>
            </div>
            <button
              className="remove-btn"
              title="移除"
              onClick={(e) => {
                e.stopPropagation();
                onRemove(s.symbol);
              }}
            >
              ×
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}