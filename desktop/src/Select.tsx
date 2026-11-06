import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { Check, ChevronDown } from "lucide-react";

export interface Option {
  value: string;
  label: string;
}

/** Themed listbox replacing the native select, whose popup follows the system theme. */
export function Select({
  label,
  value,
  options,
  onChange,
  disabled,
  className = "",
}: {
  label: string;
  value: string;
  options: Option[];
  onChange: (value: string) => void;
  disabled?: boolean;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const [place, setPlace] = useState<{
    left: number;
    top?: number;
    bottom?: number;
    width: number;
    maxHeight: number;
  } | null>(null);
  const button = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const id = useId();
  const current = options.find((o) => o.value === value) ?? options[0];

  useLayoutEffect(() => {
    if (!open || !button.current) return;
    const r = button.current.getBoundingClientRect();
    const below = window.innerHeight - r.bottom - 8;
    const above = r.top - 8;
    const wanted = Math.min(options.length * 34 + 8, 320);
    const up = below < wanted && above > below;
    setPlace({
      left: Math.max(
        8,
        Math.min(r.left, window.innerWidth - Math.max(r.width, 180) - 8),
      ),
      width: Math.max(r.width, 180),
      ...(up
        ? { bottom: window.innerHeight - r.top + 4 }
        : { top: r.bottom + 4 }),
      maxHeight: Math.max(120, Math.min(wanted, up ? above : below)),
    });
  }, [open, options.length]);
  useEffect(() => {
    if (!open) return;
    const away = (e: PointerEvent) => {
      const t = e.target as Node;
      if (!button.current?.contains(t) && !list.current?.contains(t))
        setOpen(false);
    };
    const close = () => setOpen(false);
    document.addEventListener("pointerdown", away);
    window.addEventListener("resize", close);
    return () => {
      document.removeEventListener("pointerdown", away);
      window.removeEventListener("resize", close);
    };
  }, [open]);
  useEffect(() => {
    if (open)
      list.current
        ?.querySelector<HTMLElement>("[data-active=true]")
        ?.scrollIntoView({ block: "nearest" });
  }, [open, active]);

  function choose(index: number) {
    onChange(options[index].value);
    setOpen(false);
    button.current?.focus();
  }
  function key(e: React.KeyboardEvent) {
    const selected = Math.max(
      0,
      options.findIndex((o) => o.value === value),
    );
    if (!open) {
      if (["ArrowDown", "ArrowUp", "Enter", " "].includes(e.key)) {
        e.preventDefault();
        setActive(selected);
        setOpen(true);
      }
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      setOpen(false);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((i) => Math.min(options.length - 1, i + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((i) => Math.max(0, i - 1));
    } else if (e.key === "Home") {
      e.preventDefault();
      setActive(0);
    } else if (e.key === "End") {
      e.preventDefault();
      setActive(options.length - 1);
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      choose(active);
    } else if (e.key === "Tab") setOpen(false);
  }
  return (
    <>
      <button
        ref={button}
        type="button"
        role="combobox"
        className={`select ${className}`}
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={open ? id : undefined}
        disabled={disabled}
        onClick={() => {
          setActive(
            Math.max(
              0,
              options.findIndex((o) => o.value === value),
            ),
          );
          setOpen(!open);
        }}
        onKeyDown={key}
      >
        <span>{current?.label}</span>
        <ChevronDown size={14} />
      </button>
      {open && place && (
        <ul
          ref={list}
          id={id}
          role="listbox"
          aria-label={label}
          className="select-list"
          style={place}
        >
          {options.map((o, i) => (
            <li
              key={o.value}
              role="option"
              aria-selected={o.value === value}
              data-active={i === active}
              onPointerEnter={() => setActive(i)}
              onClick={() => choose(i)}
            >
              <span>{o.label}</span>
              {o.value === value && <Check size={14} />}
            </li>
          ))}
        </ul>
      )}
    </>
  );
}
