import { useState, useCallback, type DragEvent } from "react";
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
  /** Called when the user drags a stock to reorder. */
  onReorder?: (fromIndex: number, toIndex: number) => void;
}

/** Per-item drop-zone side: top half → "before", bottom half → "after". */
type DropSide = "before" | "after";

interface DropTarget {
  idx: number;
  side: DropSide;
}

function dropTargetKey(t: DropTarget): string {
  return `${t.idx}:${t.side}`;
}

export default function StockList({
  stocks,
  activeSymbol,
  onSelect,
  onRemove,
  onReorder,
}: Props) {
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [dropTarget, setDropTarget] = useState<DropTarget | null>(null);

  const resolveDropIndex = useCallback(
    (t: DropTarget, from: number): number => {
      // toIndex means "insert before this index".
      // "before idx" → toIndex = idx
      // "after idx"  → toIndex = idx + 1
      let to = t.side === "before" ? t.idx : t.idx + 1;
      // If the source is before the target and we're inserting "after",
      // the removal already shifted the array — compensate.
      if (from < to) to -= 1;
      return to;
    },
    []
  );

  function handleDragStart(e: DragEvent, idx: number) {
    setDragIndex(idx);
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", String(idx));
  }

  function handleDragOver(e: DragEvent, idx: number) {
    e.preventDefault();
    if (dragIndex === null) return;
    e.dataTransfer.dropEffect = "move";

    // Determine which half of the item the cursor is in
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const midY = rect.top + rect.height / 2;
    const side: DropSide = e.clientY < midY ? "before" : "after";

    setDropTarget((prev) => {
      const next: DropTarget = { idx, side };
      // Skip self
      if (dragIndex === idx) return prev;
      // No-op if the same spot
      if (prev && prev.idx === idx && prev.side === side) return prev;
      return next;
    });
  }

  function handleDrop(e: DragEvent, idx: number) {
    e.preventDefault();
    if (dragIndex === null || !onReorder) {
      setDragIndex(null);
      setDropTarget(null);
      return;
    }

    // Compute drop-side from final mouse position
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const midY = rect.top + rect.height / 2;
    const side: DropSide = e.clientY < midY ? "before" : "after";
    const to = resolveDropIndex({ idx, side }, dragIndex);

    if (dragIndex !== to) {
      onReorder(dragIndex, to);
    }
    setDragIndex(null);
    setDropTarget(null);
  }

  function handleDragEnd() {
    setDragIndex(null);
    setDropTarget(null);
  }

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

        {stocks.map((s, idx) => {
          // Don't show drop-indicator on the dragged item itself
          const isDrop =
            dropTarget !== null &&
            idx === dropTarget.idx &&
            idx !== dragIndex;

          return (
            <div
              key={s.symbol}
              className={
                `list-item` +
                `${s.symbol === activeSymbol ? " active" : ""}` +
                `${idx === dragIndex ? " dragging" : ""}` +
                `${isDrop && dropTarget!.side === "before" ? " drop-before" : ""}` +
                `${isDrop && dropTarget!.side === "after" ? " drop-after" : ""}`
              }
              onClick={() => onSelect(s.symbol)}
              draggable
              onDragStart={(e) => handleDragStart(e, idx)}
              onDragOver={(e) => handleDragOver(e, idx)}
              onDrop={(e) => handleDrop(e, idx)}
              onDragEnd={handleDragEnd}
            >
              <div className="drag-handle" title="拖拽排序" />
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
          );
        })}
      </div>
    </div>
  );
}