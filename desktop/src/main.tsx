import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { createRoot } from "react-dom/client";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import {
  ArrowDownToLine,
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  BarChart3,
  Check,
  ChevronLeft,
  ChevronRight,
  CircleHelp,
  FileUp,
  FolderOpen,
  Info,
  Leaf,
  LoaderCircle,
  MapPin,
  Moon,
  RefreshCw,
  Terminal,
  Search,
  ShieldCheck,
  Sun,
  Table2,
  TriangleAlert,
  X,
} from "lucide-react";
import type {
  ExportResult,
  Location,
  Measurement,
  SourceSummary,
  WorkDetail,
} from "./types";
import { ChartPanel } from "./ChartPanel";
import { MapPanel, type MapFocus } from "./MapPanel";
import { Select } from "./Select";
import {
  ColumnHeader,
  isActive,
  matches,
  type ColumnFilterValue,
} from "./ColumnFilter";
import type { MapPoint } from "./geo";
import { warningMessage } from "./warnings";
import "./styles.css";
const number = (n: number) => n.toLocaleString("pt-BR");
const date = (s: string | null) => (s ? s.split("-").reverse().join("/") : "—");
const value = (s: string | number | null | undefined) =>
  s == null ? "—" : String(s).replace(".", ",");
const PAGE_SIZE = 75;
type Row = Measurement & {
  group: number;
  group_index: number;
  source_file: string;
  index: number;
};
type SortKey =
  | "id"
  | "group"
  | "point"
  | "date"
  | "chlorophyll_a_index"
  | "chlorophyll_b_index"
  | "chlorophyll_total_index"
  | "warnings";
const tabs = [
  { id: "table", name: "Medições", icon: Table2 },
  { id: "map", name: "Mapa", icon: MapPin },
  { id: "chart", name: "Gráficos", icon: BarChart3 },
  { id: "source", name: "Metadados", icon: ShieldCheck },
  { id: "export", name: "Exportação", icon: ArrowDownToLine },
];
const COLUMNS: [SortKey, string, "number" | "text" | "warnings"][] = [
  ["id", "ID", "number"],
  ["group", "Grupo", "number"],
  ["point", "Ponto", "number"],
  ["date", "Data / hora", "text"],
  ["chlorophyll_a_index", "Índice A", "number"],
  ["chlorophyll_b_index", "Índice B", "number"],
  ["chlorophyll_total_index", "Total", "number"],
  ["warnings", "Avisos", "warnings"],
];
const EXPORT_MODES = [
  {
    id: "simple",
    title: "Simples",
    text: "Planilhas XLSX por trabalho, prontas para análise.",
  },
  {
    id: "complete",
    title: "Completo",
    text: "Planilhas, auditoria e bytes preservados para rastreabilidade.",
  },
  {
    id: "json",
    title: "JSON de auditoria",
    text: "Estrutura, registros e origem de um único trabalho em um arquivo.",
  },
] as const;
const toPoint = (l: Location): MapPoint => ({
  key: `${l.work}-${l.group_index}-${l.record}`,
  work: l.work,
  workName: l.work_name,
  workNumber: l.work_number,
  groupIndex: l.group_index,
  group: l.group,
  record: l.record,
  id: l.id,
  point: l.point,
  date: l.date,
  time: l.time,
  a: l.a,
  b: l.b,
  total: l.total,
  lat: l.latitude,
  lon: l.longitude,
  warnings: l.warnings,
});
function App() {
  const [source, setSource] = useState<SourceSummary | null>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [active, setActive] = useState<number | null>(null);
  const [opened, setOpened] = useState(false);
  const [detail, setDetail] = useState<WorkDetail | null>(null);
  const [detailError, setDetailError] = useState("");
  const [detailAttempt, setDetailAttempt] = useState(0);
  const [locations, setLocations] = useState<MapPoint[]>([]);
  const [selKey, setSelKey] = useState<string | null>(null);
  const [focus, setFocus] = useState<MapFocus | null>(null);
  const [tab, setTab] = useState("table");
  const [search, setSearch] = useState("");
  const [query, setQuery] = useState("");
  const [warningsOnly, setWarningsOnly] = useState(false);
  const [sort, setSort] = useState<{
    key: SortKey;
    descending: boolean;
  } | null>(null);
  const [page, setPage] = useState(0);
  const [colFilters, setColFilters] = useState<
    Partial<Record<SortKey, ColumnFilterValue>>
  >({});
  const [group, setGroup] = useState("all");
  const [mode, setMode] = useState("simple");
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [result, setResult] = useState<ExportResult | null>(null);
  const [record, setRecord] = useState<Row | null>(null);
  const [help, setHelp] = useState(false);
  const [drag, setDrag] = useState(false);
  const [workSort, setWorkSort] = useState<"number" | "name" | "records">(
    "number",
  );
  const [workDescending, setWorkDescending] = useState(false);
  const [theme, setTheme] = useState(() => {
    try {
      return localStorage.getItem("fc2-theme") || "light";
    } catch {
      return "light";
    }
  });
  const request = useRef(0);
  const fileToken = useRef(0);
  const operation = useRef(false);
  const focusId = useRef(0);
  const selectedWorks = source?.works.filter((w) => selected.has(w.key)) || [];
  const activeWork = source?.works.find((w) => w.key === active);
  const rows: Row[] = useMemo(
    () =>
      detail?.groups.flatMap((g, groupIndex) =>
        g.records.map((r, index) => ({
          ...r,
          group: g.group,
          group_index: groupIndex,
          source_file: g.source_file,
          index,
        })),
      ) || [],
    [detail],
  );
  const filteredOriginRows = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase();
    return rows.filter(
      (r) =>
        (group === "all" || String(r.group) === group) &&
        (!warningsOnly || r.warnings.length > 0) &&
        (!needle ||
          `${r.id} ${r.point} ${r.date || ""} ${date(r.date)} ${r.time || ""}`
            .toLocaleLowerCase()
            .includes(needle)) &&
        COLUMNS.every(([key, , kind]) => {
          const f = colFilters[key];
          if (!isActive(f)) return true;
          if (kind === "warnings") return matches(f, null, r.warnings.length);
          if (key === "date")
            return matches(
              f,
              `${r.date || ""} ${date(r.date)} ${r.time || ""}`,
            );
          return matches(f, r[key] as string | number);
        }),
    );
  }, [rows, group, warningsOnly, query, colFilters]);
  const activeFilters = Object.values(colFilters).filter(isActive).length;
  const filteredRows = useMemo(() => {
    const filtered = [...filteredOriginRows];
    if (sort)
      filtered.sort((a, b) => {
        const k = sort.key;
        let diff: number;
        if (k === "date")
          diff = `${a.date || ""} ${a.time || ""}`.localeCompare(
            `${b.date || ""} ${b.time || ""}`,
          );
        else if (k === "warnings") diff = a.warnings.length - b.warnings.length;
        else diff = Number(a[k]) - Number(b[k]);
        return sort.descending ? -diff : diff;
      });
    return filtered;
  }, [filteredOriginRows, sort]);
  const rowKey = (r: Row) => `${active}-${r.group_index}-${r.index}`;
  const rowPoint = useCallback(
    (r: Row): MapPoint | null =>
      r.latitude == null || r.longitude == null || !detail || active == null
        ? null
        : {
            key: `${active}-${r.group_index}-${r.index}`,
            work: active,
            workName: detail.display_name,
            workNumber: detail.number,
            groupIndex: r.group_index,
            group: r.group,
            record: r.index,
            id: r.id,
            point: r.point,
            date: r.date,
            time: r.time,
            a: r.chlorophyll_a_index,
            b: r.chlorophyll_b_index,
            total: r.chlorophyll_total_index,
            lat: r.latitude,
            lon: r.longitude,
            warnings: r.warnings,
          },
    [detail, active],
  );
  const workPoints = useMemo(
    () =>
      filteredOriginRows.map(rowPoint).filter((p): p is MapPoint => p != null),
    [filteredOriginRows, rowPoint],
  );
  const works = useMemo(() => {
    const needle = search.toLocaleLowerCase();
    const list =
      source?.works.filter((w) =>
        `${w.name} ${w.number}`.toLocaleLowerCase().includes(needle),
      ) || [];
    list.sort((a, b) => {
      const diff =
        workSort === "name"
          ? a.name.localeCompare(b.name, "pt-BR", { numeric: true })
          : workSort === "records"
            ? a.records - b.records
            : a.number - b.number;
      return (workDescending ? -diff : diff) || a.key - b.key;
    });
    return list;
  }, [source, search, workSort, workDescending]);
  const markedWorks = works.filter((w) => selected.has(w.key));
  const otherWorks = works.filter((w) => !selected.has(w.key));
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem("fc2-theme", theme);
    } catch {
      /* preference storage is optional */
    }
  }, [theme]);
  const frame = useCallback((work: number | null, from: MapPoint[]) => {
    if (work == null) return;
    setFocus({
      id: ++focusId.current,
      points: from.filter((p) => p.work === work),
    });
  }, []);
  const open = useCallback(async (path?: string) => {
    if (operation.current) return;
    if (!isTauri()) {
      setError("Seleção de arquivos disponível no aplicativo desktop.");
      return;
    }
    operation.current = true;
    setBusy("Lendo FC2 e verificando a estrutura…");
    setError("");
    try {
      const data = await invoke<SourceSummary | null>("open_source", {
        path: path || null,
      });
      if (data) {
        request.current++;
        setSource(data);
        setSelected(new Set());
        setActive(null);
        setOpened(false);
        setSelKey(null);
        setDetail(null);
        setResult(null);
        setSearch("");
        setTab("table");
        setFocus(null);
        setLocations([]);
        const token = ++fileToken.current;
        invoke<Location[]>("get_locations", { source: data.sha256 })
          .then((list) => {
            // Ignore a late answer for a file that was replaced meanwhile.
            if (token === fileToken.current) setLocations(list.map(toPoint));
          })
          .catch((e) => {
            if (token === fileToken.current) setError(String(e));
          });
      }
    } catch (e) {
      setError(String(e));
    } finally {
      operation.current = false;
      setBusy("");
    }
  }, []);
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    let cleanup: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "over" || event.payload.type === "enter")
          setDrag(true);
        else {
          setDrag(false);
          if (event.payload.type === "drop" && event.payload.paths[0])
            void open(event.payload.paths[0]);
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else cleanup = fn;
      })
      .catch((e) => setError(String(e)));
    return () => {
      disposed = true;
      cleanup?.();
    };
  }, [open]);
  useEffect(() => {
    if (isTauri())
      invoke<string | null>("initial_path")
        .then((p) => {
          if (p) void open(p);
        })
        .catch((e) => setError(String(e)));
  }, [open]);
  useEffect(() => {
    const listener = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "o") {
        e.preventDefault();
        void open();
      }
      if (e.key === "Escape") {
        setRecord(null);
        setHelp(false);
      }
    };
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, [open]);
  useEffect(() => {
    if (active == null || !source) return;
    const id = ++request.current;
    setDetail(null);
    setDetailError("");
    setPage(0);
    setGroup("all");
    setRecord(null);
    setQuery("");
    setWarningsOnly(false);
    setColFilters({});
    setSort(null);
    invoke<WorkDetail>("get_work", { source: source.sha256, key: active })
      .then((data) => {
        if (id === request.current) setDetail(data);
      })
      .catch((e) => {
        if (id === request.current) setDetailError(String(e));
      });
  }, [active, source, detailAttempt]);
  function toggle(key: number) {
    setSelected((old) => {
      const next = new Set(old);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
    setResult(null);
  }
  function selectWork(key: number) {
    setActive(key);
    setSelKey(null);
    frame(key, locations);
  }
  function openWork(key: number) {
    if (key !== active) setActive(key);
    setSelKey(null);
    setOpened(true);
    setTab("table");
  }
  function back() {
    setOpened(false);
    setNotice("");
    frame(active, locations);
  }
  function selectPoint(p: MapPoint) {
    if (p.work !== active) setActive(p.work);
    setSelKey(p.key);
    // The list must show the work the point belongs to, even when filtered out.
    const w = source?.works.find((x) => x.key === p.work);
    if (
      w &&
      search &&
      !`${w.name} ${w.number}`
        .toLocaleLowerCase()
        .includes(search.toLocaleLowerCase())
    )
      setSearch("");
  }
  useEffect(() => {
    if (opened) return;
    document
      .querySelector(".work-list .work-item.active")
      ?.scrollIntoView({ block: "nearest" });
  }, [active, opened, search, selected, workSort, workDescending]);
  function sortBy(key: SortKey) {
    setSort((old) => ({
      key,
      descending: old?.key === key ? !old.descending : false,
    }));
    setPage(0);
  }
  async function exportData(keys: number[]) {
    if (!source || !keys.length || operation.current) return;
    if (mode === "json" && keys.length !== 1) {
      setError("Marque um único trabalho para exportar JSON.");
      return;
    }
    operation.current = true;
    setBusy("Validando a origem e exportando…");
    setError("");
    setResult(null);
    try {
      setResult(
        await invoke<ExportResult | null>("export_selection", {
          source: source.sha256,
          keys,
          mode,
        }),
      );
    } catch (e) {
      setError(String(e));
    } finally {
      operation.current = false;
      setBusy("");
    }
  }
  async function openTui() {
    if (!isTauri()) {
      setError("A TUI está disponível no aplicativo desktop.");
      return;
    }
    try {
      await invoke("open_tui", { path: source?.path ?? null });
      setNotice("TUI aberta em um terminal separado.");
    } catch (e) {
      setError(String(e));
    }
  }
  const clearFocus = useCallback(() => setFocus(null), []);
  function showOnMap(r: Row) {
    const point = rowPoint(r);
    if (!point) return;
    setSelKey(point.key);
    setFocus({ id: ++focusId.current, points: [point] });
    setRecord(null);
    setOpened(true);
    setTab("map");
  }
  function inspectSelected() {
    const row = rows.find((r) => rowKey(r) === selKey);
    if (row) setRecord(row);
  }
  function tableHead(
    label: string,
    key: SortKey,
    kind: "number" | "text" | "warnings",
  ) {
    return (
      <th
        key={key}
        aria-sort={
          sort?.key === key
            ? sort.descending
              ? "descending"
              : "ascending"
            : "none"
        }
      >
        <ColumnHeader
          label={label}
          kind={kind}
          sorted={
            sort?.key === key
              ? sort.descending
                ? "descending"
                : "ascending"
              : "none"
          }
          onSort={() => sortBy(key)}
          filter={colFilters[key]}
          onFilter={(f) => {
            setColFilters((old) => ({ ...old, [key]: f }));
            setPage(0);
          }}
        />
      </th>
    );
  }
  const resultNotice = result && (
    <div
      className={`notice export-result ${result.failures ? "warning" : "success"}`}
      role="status"
    >
      <div>
        <strong>
          {result.failures
            ? "Exportação com pendências"
            : "Exportação concluída"}
        </strong>
        <p>
          {mode === "json"
            ? "Auditoria JSON salva."
            : `${result.workbooks} planilhas; ${result.failures} trabalhos bloqueados.`}
        </p>
        <code>{result.path}</code>
        {result.errors.map((s, i) => (
          <p key={i}>{s}</p>
        ))}
      </div>
    </div>
  );
  const exportModeSelect = (
    <Select
      label="Formato de exportação"
      value={mode}
      disabled={!!busy}
      onChange={(v) => {
        setMode(v);
        setResult(null);
      }}
      options={[
        { value: "simple", label: "Simples: XLSX" },
        { value: "complete", label: "Completo: XLSX + auditoria" },
        { value: "json", label: "JSON de auditoria (um trabalho)" },
      ]}
    />
  );
  const selectedPoint = selKey
    ? (locations.find((p) => p.key === selKey) ??
      workPoints.find((p) => p.key === selKey) ??
      null)
    : null;
  const firstPoint =
    active != null ? locations.find((p) => p.work === active) : undefined;
  function workItem(w: SourceSummary["works"][number]) {
    return (
      <div
        key={w.key}
        className={`work-item ${active === w.key ? "active" : ""}`}
        onDoubleClick={() => openWork(w.key)}
      >
        <input
          aria-label={`Marcar ${w.name}`}
          type="checkbox"
          checked={selected.has(w.key)}
          onChange={() => {
            toggle(w.key);
            selectWork(w.key);
          }}
        />
        <button
          className="work-button"
          aria-pressed={active === w.key}
          aria-label={`Trabalho ${w.name}`}
          onClick={() => selectWork(w.key)}
        >
          <span className="work-name">
            {w.number} · {w.name}
          </span>
          <span className="work-counts">
            {number(w.records)} {w.records === 1 ? "registro" : "registros"} ·{" "}
            {w.groups} {w.groups === 1 ? "grupo" : "grupos"}
          </span>
        </button>
        {(w.errors.length > 0 || w.warnings > 0) && (
          <span
            className={`work-warning ${w.errors.length ? "danger" : ""}`}
            title={`${w.errors.length} erros; ${w.warnings} avisos`}
          >
            <TriangleAlert size={13} />
          </span>
        )}
      </div>
    );
  }
  return (
    <div className="app-shell">
      <header className="topbar">
        {source && opened && (
          <button className="button" onClick={back}>
            <ArrowLeft size={16} />
            Voltar
          </button>
        )}
        <div className="brand">
          <Leaf size={18} />
          <strong>Leaf Explorer</strong>
        </div>
        <button
          className="button"
          onClick={() => void open()}
          disabled={!!busy}
        >
          <FolderOpen size={16} />
          Abrir FC2
        </button>
        {source && (
          <button
            className="icon-button"
            aria-label="Recarregar o FC2 do disco"
            title="Recarregar o FC2 do disco"
            disabled={!!busy}
            onClick={() => void open(source.path)}
          >
            <RefreshCw size={16} />
          </button>
        )}
        {source && (
          <span className="header-source" title={source.path}>
            {source.name}
          </span>
        )}
        <div className="header-actions">
          <button
            className="icon-button"
            aria-label="Abrir TUI"
            title="Abrir a TUI em um terminal separado"
            onClick={() => void openTui()}
          >
            <Terminal size={17} />
          </button>
          <button
            className="icon-button"
            aria-label={
              theme === "light" ? "Ativar tema escuro" : "Ativar tema claro"
            }
            onClick={() => setTheme(theme === "light" ? "dark" : "light")}
          >
            {theme === "light" ? <Moon size={17} /> : <Sun size={17} />}
          </button>
          <button
            className="icon-button"
            aria-label="Ajuda"
            onClick={() => setHelp(true)}
          >
            <CircleHelp size={17} />
          </button>
        </div>
      </header>
      {(error || busy) && (
        <div
          className={`notice global-notice ${error ? "error" : "loading"}`}
          role={error ? "alert" : "status"}
        >
          {error ? (
            <TriangleAlert size={16} />
          ) : (
            <LoaderCircle size={16} className="spin" />
          )}
          <span>{error || busy}</span>
          {error && (
            <button
              className="icon-button"
              aria-label="Fechar aviso"
              onClick={() => setError("")}
            >
              <X size={15} />
            </button>
          )}
        </div>
      )}
      {!source ? (
        <main className="empty-workspace">
          <div className="open-file">
            <FolderOpen size={36} strokeWidth={1.3} />
            <h1>Abrir arquivo FC2</h1>
            <p>Trabalhos, mapa, medições e exportação.</p>
            <button
              className="button primary"
              disabled={!!busy}
              onClick={() => void open()}
            >
              <FolderOpen size={16} />
              Selecionar arquivo FC2
            </button>
            <small>
              Arraste um arquivo para a janela ou use <kbd>Ctrl O</kbd>.
            </small>
          </div>
        </main>
      ) : !opened || !activeWork ? (
        <div className="explorer-layout">
          <aside className="work-sidebar" aria-label="Trabalhos">
            <div className="sidebar-title">
              <h2>
                Trabalhos <span>{source.works.length}</span>
              </h2>
            </div>
            <label className="search">
              <Search size={15} />
              <input
                aria-label="Buscar trabalhos"
                placeholder="Nome ou identificador"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
              {search && (
                <button
                  className="icon-button"
                  aria-label="Limpar busca"
                  onClick={() => setSearch("")}
                >
                  <X size={13} />
                </button>
              )}
            </label>
            <div className="sort-toolbar">
              <Select
                label="Ordenar trabalhos"
                value={workSort}
                onChange={(v) => setWorkSort(v as typeof workSort)}
                options={[
                  { value: "number", label: "Ordenar: ID" },
                  { value: "name", label: "Ordenar: nome" },
                  { value: "records", label: "Ordenar: registros" },
                ]}
              />
              <label>
                <input
                  type="checkbox"
                  checked={workDescending}
                  onChange={(e) => setWorkDescending(e.target.checked)}
                />
                Decrescente
              </label>
            </div>
            <div className="selection-toolbar">
              <label title="Selecionar ou desmarcar todos os trabalhos do arquivo, inclusive fora da busca">
                <input
                  type="checkbox"
                  aria-label="Marcar todos os trabalhos"
                  checked={
                    source.works.length > 0 &&
                    selected.size === source.works.length
                  }
                  onChange={(e) => {
                    setSelected(
                      e.target.checked
                        ? new Set(source.works.map((w) => w.key))
                        : new Set(),
                    );
                    setResult(null);
                  }}
                />
                Todos
              </label>
              {selected.size > 0 && (
                <button
                  className="text-button"
                  onClick={() => {
                    setSelected(new Set());
                    setResult(null);
                  }}
                >
                  Limpar marcações
                </button>
              )}
            </div>
            <p className="list-hint">Clique seleciona; duplo clique abre.</p>
            <div className="work-list">
              {markedWorks.length > 0 && (
                <>
                  <h3 className="list-section">
                    Selecionados ({markedWorks.length})
                  </h3>
                  {markedWorks.map(workItem)}
                  {otherWorks.length > 0 && <hr className="list-divider" />}
                </>
              )}
              {otherWorks.map(workItem)}
              {!works.length && (
                <p className="list-empty">Nenhum trabalho encontrado.</p>
              )}
            </div>
            <div className="sidebar-summary">
              <span>
                {works.length} visíveis • {selected.size} marcados
              </span>
              {exportModeSelect}
              <button
                className="button primary"
                disabled={!selected.size || !!busy}
                onClick={() => void exportData([...selected])}
              >
                {busy.startsWith("Validando") ? (
                  <LoaderCircle size={15} className="spin" />
                ) : (
                  <ArrowDownToLine size={15} />
                )}
                Exportar marcados
              </button>
              {resultNotice}
            </div>
          </aside>
          <main className="explorer-map" aria-label="Visão geográfica">
            <MapPanel
              title="Visão geográfica"
              points={locations}
              region={locations}
              selectedKey={selKey}
              relatedWork={active}
              onSelect={selectPoint}
              focus={focus}
              onFocused={clearFocus}
              fitKey={`all|${locations.length}`}
              fill
            />
          </main>
          <aside className="explorer-details" aria-label="Detalhes do trabalho">
            {activeWork ? (
              <>
                <h2>{activeWork.name}</h2>
                <p>
                  ID {activeWork.number} • {number(activeWork.records)}{" "}
                  registros • {activeWork.groups} grupos
                </p>
                <p>
                  {number(activeWork.coordinates)} coordenadas numéricas •{" "}
                  {number(activeWork.warnings)}{" "}
                  {activeWork.warnings === 1
                    ? "registro com aviso"
                    : "registros com avisos"}
                </p>
                <p
                  className={`badge ${activeWork.errors.length ? "danger" : "success"}`}
                >
                  {activeWork.errors.length ? (
                    <TriangleAlert size={12} />
                  ) : (
                    <Check size={12} />
                  )}
                  {activeWork.errors.length
                    ? "Estrutura com erros; exportação pode ser bloqueada"
                    : "Estrutura interpretada"}
                </p>
                <small>
                  Equivalência Leaf não presumida. Datum GPS não confirmado.
                </small>
                <code className="directory">{activeWork.directory}</code>
                {selectedPoint && selectedPoint.work === active ? (
                  <div className="details-block">
                    <strong>
                      Registro {selectedPoint.id} • grupo {selectedPoint.group}
                    </strong>
                    <span>Latitude {selectedPoint.lat}</span>
                    <span>Longitude {selectedPoint.lon}</span>
                    <button
                      className="button"
                      disabled={!detail}
                      onClick={inspectSelected}
                    >
                      <Info size={15} />
                      Inspecionar registro
                    </button>
                  </div>
                ) : (
                  firstPoint && (
                    <div className="details-block">
                      <small>
                        Primeira coordenada disponível na ordem do arquivo:
                      </small>
                      <span>Latitude {firstPoint.lat}</span>
                      <span>Longitude {firstPoint.lon}</span>
                      <button
                        className="button"
                        onClick={() => {
                          setSelKey(firstPoint.key);
                          setFocus({
                            id: ++focusId.current,
                            points: [firstPoint],
                          });
                        }}
                      >
                        <MapPin size={15} />
                        Centralizar no mapa
                      </button>
                    </div>
                  )
                )}
                {activeWork.errors.map((s, i) => (
                  <p className="notice error" key={i}>
                    {s}
                  </p>
                ))}
                {activeWork.notes.map((s, i) => (
                  <small key={i}>{s}</small>
                ))}
                <button
                  className="button primary open-work"
                  onClick={() => openWork(activeWork.key)}
                >
                  <ArrowRight size={15} />
                  Abrir trabalho
                </button>
              </>
            ) : (
              <p className="list-empty">
                Selecione um trabalho na lista ou clique em um ponto do mapa.
              </p>
            )}
          </aside>
        </div>
      ) : (
        <main className="work-view" aria-busy={!!busy}>
          <section className="work-heading">
            <div>
              <h1>{activeWork.name}</h1>
              <span className="work-id">ID {activeWork.number}</span>
              <span
                className={`badge ${activeWork.errors.length ? "danger" : "success"}`}
              >
                {activeWork.errors.length ? (
                  <TriangleAlert size={12} />
                ) : (
                  <Check size={12} />
                )}{" "}
                {activeWork.errors.length
                  ? "Estrutura com erros"
                  : "Estrutura interpretada"}
              </span>
            </div>
            <div className="work-facts">
              <span>
                <strong>{number(activeWork.records)}</strong> registros
              </span>
              <span>
                <strong>{activeWork.groups}</strong> grupos
              </span>
              <span>
                <strong>{number(activeWork.points)}</strong> pontos
              </span>
              <span>
                <strong>{number(activeWork.coordinates)}</strong> coordenadas
              </span>
              {activeWork.warnings > 0 && (
                <span className="warning-text">
                  {activeWork.warnings}{" "}
                  {activeWork.warnings === 1
                    ? "registro com aviso"
                    : "registros com avisos"}
                </span>
              )}
              <span className="period">
                {date(activeWork.first_date)}
                {activeWork.last_date &&
                activeWork.last_date !== activeWork.first_date
                  ? ` – ${date(activeWork.last_date)}`
                  : ""}
              </span>
            </div>
          </section>
          <div
            className="tabs"
            role="tablist"
            aria-label="Visualização do trabalho"
          >
            {tabs.map((t) => (
              <button
                key={t.id}
                role="tab"
                aria-selected={tab === t.id}
                className={tab === t.id ? "active" : ""}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => setTab(t.id)}
              >
                <t.icon size={15} />
                {t.name}
              </button>
            ))}
          </div>
          {detailError ? (
            <div className="empty-state" role="alert">
              <TriangleAlert size={24} />
              <p>{detailError}</p>
              <button
                className="button"
                onClick={() => setDetailAttempt((n) => n + 1)}
              >
                Tentar novamente
              </button>
            </div>
          ) : !detail ? (
            <div className="empty-state" role="status">
              <LoaderCircle size={24} className="spin" />
              <p>Carregando medições…</p>
            </div>
          ) : (
            <div
              className={`tab-content ${tab === "table" ? "table-content" : ""} ${tab === "map" ? "map-content" : ""}`}
              role="tabpanel"
            >
              {(tab === "table" || tab === "map" || tab === "chart") && (
                <div className="record-toolbar">
                  <label className="search record-search">
                    <Search size={14} />
                    <input
                      aria-label="Buscar medições"
                      placeholder="ID, ponto, data ou hora"
                      value={query}
                      onChange={(e) => {
                        setQuery(e.target.value);
                        setPage(0);
                      }}
                    />
                  </label>
                  <Select
                    label="Filtrar grupo"
                    value={group}
                    onChange={(v) => {
                      setGroup(v);
                      setPage(0);
                    }}
                    options={[
                      { value: "all", label: "Todos os grupos" },
                      ...detail.groups.map((g) => ({
                        value: String(g.group),
                        label: `Grupo ${g.group}`,
                      })),
                    ]}
                  />
                  <label className="warning-filter">
                    <input
                      type="checkbox"
                      checked={warningsOnly}
                      onChange={(e) => {
                        setWarningsOnly(e.target.checked);
                        setPage(0);
                      }}
                    />
                    Somente avisos
                  </label>
                  <span className="record-count">
                    {number(filteredRows.length)} / {number(rows.length)}{" "}
                    registros
                  </span>
                  {activeFilters > 0 && (
                    <button
                      className="text-button"
                      onClick={() => {
                        setColFilters({});
                        setPage(0);
                      }}
                    >
                      Limpar {activeFilters}{" "}
                      {activeFilters === 1 ? "filtro" : "filtros"} de coluna
                    </button>
                  )}
                </div>
              )}
              {tab === "table" && (
                <>
                  <div className="table-scroll">
                    <table className="measure-table">
                      <thead>
                        <tr>
                          {tableHead("ID", "id", "number")}
                          {tableHead("Grupo", "group", "number")}
                          {tableHead("Ponto", "point", "number")}
                          {tableHead("Data / hora", "date", "text")}
                          {tableHead(
                            "Índice A",
                            "chlorophyll_a_index",
                            "number",
                          )}
                          {tableHead(
                            "Índice B",
                            "chlorophyll_b_index",
                            "number",
                          )}
                          {tableHead(
                            "Total",
                            "chlorophyll_total_index",
                            "number",
                          )}
                          <th>Latitude</th>
                          <th>Longitude</th>
                          {tableHead("Avisos", "warnings", "warnings")}
                        </tr>
                      </thead>
                      <tbody>
                        {filteredRows
                          .slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE)
                          .map((r) => (
                            <tr
                              key={`${r.group}-${r.index}`}
                              className={rowKey(r) === selKey ? "selected" : ""}
                              tabIndex={0}
                              title="Abrir detalhes do registro"
                              onClick={() => {
                                setSelKey(rowKey(r));
                                setRecord(r);
                              }}
                              onKeyDown={(e) => {
                                if (e.key === "Enter" || e.key === " ") {
                                  e.preventDefault();
                                  setSelKey(rowKey(r));
                                  setRecord(r);
                                }
                              }}
                            >
                              <td>{r.id}</td>
                              <td>{r.group}</td>
                              <td>{r.point}</td>
                              <td className="date-cell">
                                {date(r.date)} <span>{r.time || "—"}</span>
                              </td>
                              <td>{value(r.chlorophyll_a_index)}</td>
                              <td>{value(r.chlorophyll_b_index)}</td>
                              <td className="total-value">
                                {value(r.chlorophyll_total_index)}
                              </td>
                              <td>{r.latitude?.toFixed(6) || "—"}</td>
                              <td>{r.longitude?.toFixed(6) || "—"}</td>
                              <td
                                className={
                                  r.warnings.length ? "warning-text" : ""
                                }
                                title={r.warnings
                                  .map((w) => warningMessage(w))
                                  .join("\n")}
                              >
                                {r.warnings.length || "—"}
                              </td>
                            </tr>
                          ))}
                      </tbody>
                    </table>
                    {!filteredRows.length && (
                      <div className="empty-state">
                        <p>Nenhuma medição para os filtros atuais.</p>
                      </div>
                    )}
                  </div>
                  <div className="table-bottom">
                    <span>
                      Índices do instrumento · Ausências: — · Clique no registro
                      para inspecionar
                    </span>
                    <div className="pagination">
                      <button
                        className="icon-button"
                        aria-label="Página anterior"
                        disabled={page === 0}
                        onClick={() => setPage(page - 1)}
                      >
                        <ChevronLeft size={15} />
                      </button>
                      <span>
                        {page + 1} de{" "}
                        {Math.max(
                          1,
                          Math.ceil(filteredRows.length / PAGE_SIZE),
                        )}
                      </span>
                      <button
                        className="icon-button"
                        aria-label="Próxima página"
                        disabled={(page + 1) * PAGE_SIZE >= filteredRows.length}
                        onClick={() => setPage(page + 1)}
                      >
                        <ChevronRight size={15} />
                      </button>
                    </div>
                  </div>
                </>
              )}
              {tab === "chart" && <ChartPanel records={filteredOriginRows} />}
              {tab === "map" && (
                <MapPanel
                  title="Coordenadas dos registros"
                  points={workPoints}
                  region={locations}
                  selectedKey={selKey}
                  relatedWork={active}
                  onSelect={(p) => setSelKey(p.key)}
                  focus={focus}
                  onFocused={clearFocus}
                  fitKey={`${active}|${group}|${warningsOnly}|${query}|${JSON.stringify(colFilters)}`}
                  fill
                  action={
                    selKey && rows.some((r) => rowKey(r) === selKey) ? (
                      <button className="button" onClick={inspectSelected}>
                        <Info size={15} />
                        Inspecionar registro
                      </button>
                    ) : undefined
                  }
                />
              )}
              {tab === "source" && (
                <div className="audit-content">
                  <h3>Origem</h3>
                  <dl className="metadata">
                    <dt>Arquivo</dt>
                    <dd>{source.name}</dd>
                    <dt>Tamanho</dt>
                    <dd>{number(source.size)} bytes</dd>
                    <dt>SHA-256</dt>
                    <dd className="mono">{source.sha256}</dd>
                    <dt>Diretório</dt>
                    <dd>{detail.directory}</dd>
                    <dt>Configuração</dt>
                    <dd>{detail.configuration_file || "Não disponível"}</dd>
                    <dt>Origem do nome</dt>
                    <dd>{detail.name_source}</dd>
                  </dl>
                  <h3>Avisos e limitações</h3>
                  {[
                    ...detail.errors,
                    ...detail.warnings,
                    ...source.limitations,
                  ].map((s, i) => (
                    <p className="limitation" key={i}>
                      <Info size={14} />
                      {s}
                    </p>
                  ))}
                  <p className="quiet-note">
                    Equivalência Leaf não presumida. Datum das coordenadas não
                    confirmado.
                  </p>
                </div>
              )}
              {tab === "export" && (
                <div className="audit-content export-tab">
                  <h3>Exportar trabalho</h3>
                  <div
                    className="option-cards"
                    role="radiogroup"
                    aria-label="Formato de exportação"
                  >
                    {EXPORT_MODES.map((m) => (
                      <button
                        key={m.id}
                        role="radio"
                        aria-checked={mode === m.id}
                        className="option-card"
                        disabled={!!busy}
                        onClick={() => {
                          setMode(m.id);
                          setResult(null);
                        }}
                      >
                        <strong>{m.title}</strong>
                        <span>{m.text}</span>
                      </button>
                    ))}
                  </div>
                  <p className="export-description">
                    Ao exportar, escolha uma pasta no diálogo do sistema. XLSX
                    usa uma subpasta nova; JSON permite escolher o arquivo.
                    Arquivos existentes são preservados.
                  </p>
                  <p className="quiet-note">
                    CSV científico exige um perfil de referência validado; não
                    está disponível sem essa referência.
                  </p>
                  {resultNotice}
                  <button
                    className="button primary"
                    disabled={!!busy}
                    onClick={() => void exportData([activeWork.key])}
                  >
                    {busy ? (
                      <LoaderCircle size={15} className="spin" />
                    ) : (
                      <ArrowDownToLine size={15} />
                    )}
                    Escolher destino e exportar
                  </button>
                </div>
              )}
            </div>
          )}
        </main>
      )}
      <footer className="statusbar">
        <span>
          {source
            ? `${source.name} · ${(source.size / 1048576).toLocaleString("pt-BR", { maximumFractionDigits: 2 })} MB`
            : "Nenhum arquivo aberto"}
        </span>
        {source && (
          <span>
            CRC conferido · {number(source.crc_verified)} setores ·{" "}
            {source.works.length} trabalhos · {number(source.records)} medições
            · {number(source.coordinates)} coordenadas
          </span>
        )}
        <span className="status-right">
          {busy || notice || "Leaf Explorer 0.2.0"}
        </span>
      </footer>
      {drag && (
        <div className="drop-overlay">
          <FileUp size={38} />
          <strong>Solte o arquivo FC2</strong>
        </div>
      )}
      {record && (
        <Dialog
          title={`Medição ${record.id}`}
          close={() => setRecord(null)}
          footer={record.latitude == null || record.longitude == null}
        >
          <p>
            Grupo {record.group} · Ponto {record.point}
          </p>
          <dl className="metadata">
            <dt>Data e hora</dt>
            <dd>
              {date(record.date)} {record.time || "—"}
            </dd>
            <dt>Latitude / longitude</dt>
            <dd>
              {value(record.latitude)} / {value(record.longitude)}
            </dd>
            <dt>Arquivo</dt>
            <dd>{record.source_file}</dd>
            <dt>SHA-256 do registro</dt>
            <dd className="mono">{record.raw_sha256}</dd>
          </dl>
          <h4>Segmentos FC2</h4>
          {record.segments.map((s, i) => (
            <code className="segment" key={i}>
              Offset 0x{s.file_offset.toString(16)} · Memória 0x
              {s.memory_address.toString(16)} · {s.length} bytes
            </code>
          ))}
          <h4>Bytes preservados</h4>
          <pre className="raw-bytes">
            {record.raw_hex.match(/.{1,2}/g)?.join(" ")}
          </pre>
          {record.warnings.map((s, i) => (
            <p className="notice warning" key={i}>
              {warningMessage(s)} <code>{s}</code>
            </p>
          ))}
          {record.latitude != null && record.longitude != null && (
            <div className="dialog-footer">
              <button className="button" onClick={() => showOnMap(record)}>
                <MapPin size={15} />
                Mostrar no mapa
              </button>
              <button
                className="button primary"
                onClick={() => setRecord(null)}
              >
                Fechar
              </button>
            </div>
          )}
        </Dialog>
      )}
      {help && (
        <Dialog title="Leaf Explorer" close={() => setHelp(false)}>
          <h3 className="help-title">Como usar</h3>
          <p>
            Abra um FC2: a visão geográfica mostra todos os pontos. Clique em um
            trabalho ou em um ponto para destacá-lo; duplo clique abre o
            trabalho. As caixas à esquerda marcam os trabalhos para exportação.
          </p>
          <p>
            No trabalho, use as abas para medições, mapa, gráficos, metadados e
            exportação. Nas colunas da tabela, o nome ordena e o funil filtra.
          </p>
          <p>
            <kbd>Ctrl O</kbd> Abrir arquivo · <kbd>Esc</kbd> Fechar diálogo
          </p>
          <h3 className="help-title">Privacidade e limites</h3>
          <p>
            O fundo do mapa vem do OpenStreetMap: a região visível é consultada
            pelo aplicativo; os dados FC2 não são enviados. Sem conexão, os
            pontos continuam disponíveis.
          </p>
          <p>
            Os índices instrumentais não representam concentração em massa. O
            CRC confirma integridade estrutural, não equivalência científica.
            Equivalência Leaf não presumida; datum não confirmado.
          </p>
          <h3 className="help-title">Licenças incorporadas</h3>
          <p className="license">
            Leaf Explorer: MIT. Ícones Lucide: ISC. Fonte Ubuntu: Ubuntu Font
            Licence 1.0 (public/fonts). Mapas © OpenStreetMap contributors
            (ODbL).
          </p>
        </Dialog>
      )}
    </div>
  );
}
function Dialog({
  title,
  close,
  children,
  footer = true,
}: {
  title: string;
  close: () => void;
  children: React.ReactNode;
  footer?: boolean;
}) {
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    box.current?.focus();
    const trap = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;
      const items = box.current?.querySelectorAll<HTMLElement>(
        'button, input, select, a[href], [tabindex="0"]',
      );
      if (!items?.length) return;
      const first = items[0],
        last = items[items.length - 1];
      if (
        e.shiftKey &&
        (document.activeElement === first ||
          document.activeElement === box.current)
      ) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", trap);
    return () => {
      document.removeEventListener("keydown", trap);
      previous?.focus();
    };
  }, []);
  return (
    <div className="modal-backdrop" onClick={close}>
      <div
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        ref={box}
        onClick={(e) => e.stopPropagation()}
      >
        <header>
          <h2>{title}</h2>
          <button
            className="icon-button"
            aria-label="Fechar diálogo"
            onClick={close}
          >
            <X size={20} />
          </button>
        </header>
        {children}
        {footer && (
          <div className="dialog-footer">
            <button className="button primary" onClick={close}>
              Fechar
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
