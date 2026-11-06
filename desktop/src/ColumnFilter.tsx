import { useEffect, useRef, useState } from "react";
import { ArrowDown, ArrowUp, ArrowUpDown, Filter, X } from "lucide-react";

export type ColumnKind = "number" | "text" | "warnings";
export interface ColumnFilterValue {
  min?: string;
  max?: string;
  text?: string;
  warnings?: "with" | "without";
}
export const isActive = (f?: ColumnFilterValue) =>
  !!f && (!!f.min || !!f.max || !!f.text?.trim() || !!f.warnings);
const parse = (s?: string) => {
  const n = Number((s ?? "").replace(",", ".").trim());
  return s?.trim() && Number.isFinite(n) ? n : null;
};
/** True when `value` satisfies the column filter; empty filters accept everything. */
export function matches(
  f: ColumnFilterValue | undefined,
  value: number | string | null,
  warnings = 0,
) {
  if (!isActive(f)) return true;
  if (f!.warnings === "with" && warnings === 0) return false;
  if (f!.warnings === "without" && warnings > 0) return false;
  const text = f!.text?.trim().toLocaleLowerCase();
  if (
    text &&
    !String(value ?? "")
      .toLocaleLowerCase()
      .includes(text)
  )
    return false;
  const min = parse(f!.min);
  const max = parse(f!.max);
  if (min != null || max != null) {
    const n = Number(value);
    if (!Number.isFinite(n)) return false;
    if (min != null && n < min) return false;
    if (max != null && n > max) return false;
  }
  return true;
}

/** Column header: the label sorts, the funnel opens a filter popover. */
export function ColumnHeader({
  label,
  kind,
  sorted,
  onSort,
  filter,
  onFilter,
}: {
  label: string;
  kind: ColumnKind;
  sorted: "ascending" | "descending" | "none";
  onSort: () => void;
  filter?: ColumnFilterValue;
  onFilter: (value: ColumnFilterValue) => void;
}) {
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  const active = isActive(filter);
  useEffect(() => {
    if (!open) return;
    const away = (e: PointerEvent) => {
      if (!box.current?.contains(e.target as Node)) setOpen(false);
    };
    const esc = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", away);
    document.addEventListener("keydown", esc, true);
    return () => {
      document.removeEventListener("pointerdown", away);
      document.removeEventListener("keydown", esc, true);
    };
  }, [open]);
  const Icon =
    sorted === "ascending"
      ? ArrowUp
      : sorted === "descending"
        ? ArrowDown
        : ArrowUpDown;
  return (
    <div className="col-head" ref={box}>
      <button
        className={`col-sort ${sorted !== "none" ? "on" : ""}`}
        onClick={onSort}
        title="Ordenar por esta coluna; clique novamente para inverter"
      >
        {label}
        <Icon size={13} />
      </button>
      <button
        className={`col-filter ${active ? "on" : ""}`}
        aria-label={`Filtrar ${label}`}
        aria-expanded={open}
        title={`Filtrar ${label}`}
        onClick={() => setOpen(!open)}
      >
        <Filter size={13} />
      </button>
      {open && (
        <div
          className="col-popover"
          role="dialog"
          aria-label={`Filtro de ${label}`}
        >
          {kind === "number" && (
            <div className="range">
              <label>
                Mínimo
                <input
                  inputMode="decimal"
                  value={filter?.min ?? ""}
                  onChange={(e) => onFilter({ ...filter, min: e.target.value })}
                />
              </label>
              <label>
                Máximo
                <input
                  inputMode="decimal"
                  value={filter?.max ?? ""}
                  onChange={(e) => onFilter({ ...filter, max: e.target.value })}
                />
              </label>
            </div>
          )}
          {kind === "text" && (
            <label>
              Contém
              <input
                autoFocus
                value={filter?.text ?? ""}
                onChange={(e) => onFilter({ ...filter, text: e.target.value })}
              />
            </label>
          )}
          {kind === "warnings" && (
            <div className="segmented" role="group" aria-label="Avisos">
              {(
                [
                  [undefined, "Todos"],
                  ["with", "Com avisos"],
                  ["without", "Sem avisos"],
                ] as const
              ).map(([v, l]) => (
                <button
                  key={l}
                  aria-pressed={filter?.warnings === v}
                  onClick={() => onFilter({ warnings: v })}
                >
                  {l}
                </button>
              ))}
            </div>
          )}
          <button
            className="text-button"
            disabled={!active}
            onClick={() => onFilter({})}
          >
            <X size={12} /> Limpar filtro
          </button>
        </div>
      )}
    </div>
  );
}
