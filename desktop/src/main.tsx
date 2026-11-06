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
  ArrowUp,
  BarChart3,
  Check,
  ChevronLeft,
  ChevronRight,
  CircleHelp,
  Database,
  FileUp,
  FolderOpen,
  Info,
  LoaderCircle,
  MapPin,
  Moon,
  Search,
  ShieldCheck,
  Sun,
  Table2,
  TriangleAlert,
  X,
} from "lucide-react";
import type {
  ExportResult,
  Measurement,
  SourceSummary,
  WorkDetail,
} from "./types";
import "./styles.css";
const number = (n: number) => n.toLocaleString("pt-BR");
const date = (s: string | null) => (s ? s.split("-").reverse().join("/") : "—");
const value = (s: string | number | null | undefined) =>
  s == null ? "—" : String(s).replace(".", ",");
const PAGE_SIZE = 75;
type Row = Measurement & { group: number; source_file: string; index: number };
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
  { id: "chart", name: "Gráficos", icon: BarChart3 },
  { id: "map", name: "Coordenadas", icon: MapPin },
  { id: "source", name: "Metadados", icon: ShieldCheck },
];
function App() {
  const [source, setSource] = useState<SourceSummary | null>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [active, setActive] = useState<number | null>(null);
  const [detail, setDetail] = useState<WorkDetail | null>(null);
  const [detailError, setDetailError] = useState("");
  const [detailAttempt, setDetailAttempt] = useState(0);
  const [tab, setTab] = useState("table");
  const [search, setSearch] = useState("");
  const [query, setQuery] = useState("");
  const [warningsOnly, setWarningsOnly] = useState(false);
  const [sort, setSort] = useState<{
    key: SortKey;
    descending: boolean;
  } | null>(null);
  const [page, setPage] = useState(0);
  const [group, setGroup] = useState("all");
  const [mode, setMode] = useState("simple");
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [result, setResult] = useState<ExportResult | null>(null);
  const [record, setRecord] = useState<Row | null>(null);
  const [help, setHelp] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [drag, setDrag] = useState(false);
  const [theme, setTheme] = useState(
    () => localStorage.getItem("fc2-theme") || "light",
  );
  const request = useRef(0);
  const operation = useRef(false);
  const selectedWorks = source?.works.filter((w) => selected.has(w.key)) || [];
  const selectedRecords = selectedWorks.reduce((sum, w) => sum + w.records, 0);
  const activeWork = source?.works.find((w) => w.key === active);
  const rows: Row[] = useMemo(
    () =>
      detail?.groups.flatMap((g) =>
        g.records.map((r, index) => ({
          ...r,
          group: g.group,
          source_file: g.source_file,
          index,
        })),
      ) || [],
    [detail],
  );
  const filteredOriginRows = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase();
    const filtered = rows.filter(
      (r) =>
        (group === "all" || String(r.group) === group) &&
        (!warningsOnly || r.warnings.length > 0) &&
        (!needle ||
          `${r.id} ${r.point} ${r.date || ""} ${date(r.date)} ${r.time || ""}`
            .toLocaleLowerCase()
            .includes(needle)),
    );
    return filtered;
  }, [rows, group, warningsOnly, query]);
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
  const works =
    source?.works.filter((w) =>
      `${w.name} ${w.number}`
        .toLocaleLowerCase()
        .includes(search.toLocaleLowerCase()),
    ) || [];
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("fc2-theme", theme);
  }, [theme]);
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
        setSelected(new Set(data.works.map((w) => w.key)));
        setActive(data.works[0]?.key ?? null);
        setDetail(null);
        setResult(null);
        setSearch("");
        setTab("table");
        setExportOpen(false);
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
        if (!operation.current) setExportOpen(false);
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
  function sortBy(key: SortKey) {
    setSort((old) => ({
      key,
      descending: old?.key === key ? !old.descending : false,
    }));
    setPage(0);
  }
  async function exportData() {
    if (!source || !selected.size || operation.current) return;
    operation.current = true;
    setBusy("Validando a origem e exportando…");
    setError("");
    setResult(null);
    try {
      setResult(
        await invoke<ExportResult | null>("export_selection", {
          source: source.sha256,
          keys: [...selected],
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
  function closeExport() {
    if (!operation.current) setExportOpen(false);
  }
  function tableHead(label: string, key: SortKey) {
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
        <button onClick={() => sortBy(key)}>
          {label}
          {sort?.key === key &&
            (sort.descending ? <ArrowDown size={12} /> : <ArrowUp size={12} />)}
        </button>
      </th>
    );
  }
  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand">
          <Database size={18} />
          <strong>FC2 Explorer</strong>
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
          <span className="header-source" title={source.name}>
            {source.name}
          </span>
        )}
        <div className="header-actions">
          <button
            className="button primary"
            disabled={!source || !selected.size || !!busy}
            onClick={() => {
              setExportOpen(true);
              setResult(null);
            }}
          >
            <ArrowDownToLine size={16} />
            Exportar{source && ` (${selected.size})`}
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
            <p>Trabalhos, medições e exportação.</p>
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
      ) : (
        <div className="explorer-layout">
          <aside className="work-sidebar" aria-label="Trabalhos">
            <div className="sidebar-title">
              <h2>
                Trabalhos <span>{source.works.length}</span>
              </h2>
              <span>{selected.size} selecionados</span>
            </div>
            <label className="search">
              <Search size={15} />
              <input
                aria-label="Buscar trabalhos"
                placeholder="Nome ou número"
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
            <div className="selection-toolbar">
              <label>
                <input
                  type="checkbox"
                  aria-label="Selecionar trabalhos visíveis"
                  checked={
                    works.length > 0 && works.every((w) => selected.has(w.key))
                  }
                  onChange={(e) => {
                    const next = new Set(selected);
                    works.forEach((w) =>
                      e.target.checked ? next.add(w.key) : next.delete(w.key),
                    );
                    setSelected(next);
                    setResult(null);
                  }}
                />
                Visíveis
              </label>
              <button
                className="text-button"
                onClick={() => {
                  setSelected(new Set(source.works.map((w) => w.key)));
                  setResult(null);
                }}
              >
                Todos
              </button>
              <button
                className="text-button"
                onClick={() => {
                  setSelected(new Set());
                  setResult(null);
                }}
              >
                Limpar
              </button>
            </div>
            <div className="work-list">
              {works.map((w) => (
                <div
                  key={w.key}
                  className={`work-item ${active === w.key ? "active" : ""}`}
                >
                  <input
                    aria-label={`Selecionar ${w.name}`}
                    type="checkbox"
                    checked={selected.has(w.key)}
                    onChange={() => toggle(w.key)}
                  />
                  <button
                    className="work-button"
                    aria-pressed={active === w.key}
                    aria-label={`Abrir trabalho ${w.name}`}
                    onClick={() => setActive(w.key)}
                  >
                    <span className="work-name">{w.name}</span>
                    <span className="work-counts">
                      ID {w.number} · {number(w.records)} medições · {w.groups}{" "}
                      gr.
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
              ))}
              {!works.length && (
                <p className="list-empty">Nenhum trabalho encontrado.</p>
              )}
            </div>
            <div className="sidebar-summary">
              <span>
                {number(source.records)} medições · {source.groups} grupos
              </span>
              <span>{number(selectedRecords)} medições selecionadas</span>
            </div>
          </aside>
          <main className="work-main" aria-busy={!!busy}>
            {activeWork ? (
              <>
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
                      <strong>{number(activeWork.records)}</strong> medições
                    </span>
                    <span>
                      <strong>{activeWork.groups}</strong> grupos
                    </span>
                    <span>
                      <strong>{number(activeWork.points)}</strong> pontos
                    </span>
                    <span>
                      <strong>{number(activeWork.coordinates)}</strong> com
                      coordenadas
                    </span>
                    {activeWork.warnings > 0 && (
                      <span className="warning-text">
                        {activeWork.warnings}{" "}
                        {activeWork.warnings === 1 ? "aviso" : "avisos"}
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
                    className={`tab-content ${tab === "table" ? "table-content" : ""}`}
                    role="tabpanel"
                  >
                    {tab !== "source" && (
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
                        <select
                          aria-label="Filtrar grupo"
                          value={group}
                          onChange={(e) => {
                            setGroup(e.target.value);
                            setPage(0);
                          }}
                        >
                          <option value="all">Todos os grupos</option>
                          {detail.groups.map((g) => (
                            <option key={g.group} value={g.group}>
                              Grupo {g.group}
                            </option>
                          ))}
                        </select>
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
                      </div>
                    )}
                    {tab === "table" && (
                      <>
                        <div className="table-scroll">
                          <table className="measure-table">
                            <thead>
                              <tr>
                                {tableHead("ID", "id")}
                                {tableHead("Grupo", "group")}
                                {tableHead("Ponto", "point")}
                                {tableHead("Data / hora", "date")}
                                {tableHead("Índice A", "chlorophyll_a_index")}
                                {tableHead("Índice B", "chlorophyll_b_index")}
                                {tableHead("Total", "chlorophyll_total_index")}
                                <th>Latitude</th>
                                <th>Longitude</th>
                                {tableHead("Avisos", "warnings")}
                                <th />
                              </tr>
                            </thead>
                            <tbody>
                              {filteredRows
                                .slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE)
                                .map((r) => (
                                  <tr
                                    key={`${r.group}-${r.index}`}
                                    onClick={() => setRecord(r)}
                                  >
                                    <td>{r.id}</td>
                                    <td>{r.group}</td>
                                    <td>{r.point}</td>
                                    <td className="date-cell">
                                      {date(r.date)}{" "}
                                      <span>{r.time || "—"}</span>
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
                                    >
                                      {r.warnings.length || "—"}
                                    </td>
                                    <td>
                                      <button
                                        className="icon-button inspect-button"
                                        aria-label={`Inspecionar medição ${r.id} do grupo ${r.group}`}
                                        onClick={(e) => {
                                          e.stopPropagation();
                                          setRecord(r);
                                        }}
                                      >
                                        <Info size={14} />
                                      </button>
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
                            Índices do instrumento · Ausências: — · Clique no
                            registro para inspecionar
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
                              disabled={
                                (page + 1) * PAGE_SIZE >= filteredRows.length
                              }
                              onClick={() => setPage(page + 1)}
                            >
                              <ChevronRight size={15} />
                            </button>
                          </div>
                        </div>
                      </>
                    )}
                    {tab === "chart" && <Chart rows={filteredOriginRows} />}
                    {tab === "map" && (
                      <CoordinatePlot rows={filteredOriginRows} />
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
                          <dd>
                            {detail.configuration_file || "Não disponível"}
                          </dd>
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
                          Equivalência Leaf não presumida. Datum das coordenadas
                          não confirmado.
                        </p>
                      </div>
                    )}
                  </div>
                )}
              </>
            ) : (
              <div className="empty-state">
                <Database size={28} />
                <p>Nenhum trabalho ativo no arquivo.</p>
              </div>
            )}
          </main>
        </div>
      )}
      <footer className="statusbar">
        <span>
          {source
            ? `${source.name} · ${(source.size / 1048576).toLocaleString("pt-BR", { maximumFractionDigits: 2 })} MB`
            : "Nenhum arquivo aberto"}
        </span>
        {source && (
          <span>
            {source.works.length} trabalhos · {number(source.records)} medições
            · {number(source.coordinates)} coordenadas
          </span>
        )}
        <span className="status-right">
          {busy || "Local · FC2 Explorer 0.2.0"}
        </span>
      </footer>
      {drag && (
        <div className="drop-overlay">
          <FileUp size={38} />
          <strong>Solte o arquivo FC2</strong>
        </div>
      )}
      {exportOpen && source && (
        <Dialog title="Exportar trabalhos" close={closeExport} footer={false}>
          <div className="export-selection">
            <strong>
              {selected.size} {selected.size === 1 ? "trabalho" : "trabalhos"}
            </strong>
            <span>{number(selectedRecords)} medições</span>
          </div>
          <label className="field-label">
            Formato
            <select
              aria-label="Formato de exportação"
              value={mode}
              disabled={!!busy}
              onChange={(e) => {
                setMode(e.target.value);
                setResult(null);
              }}
            >
              <option value="simple">XLSX — somente planilhas</option>
              <option value="complete">Completo — planilhas e auditoria</option>
              <option value="json">JSON — auditoria de um trabalho</option>
            </select>
          </label>
          <p className="export-description">
            {mode === "json"
              ? "Um arquivo JSON com registros, bytes e origem."
              : "Uma nova subpasta será criada no destino escolhido. Arquivos existentes são preservados."}
          </p>
          {mode === "json" && selected.size !== 1 && (
            <p className="notice warning">
              Selecione um único trabalho para exportar JSON.
            </p>
          )}
          {result && (
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
          )}
          {error && (
            <p className="notice error" role="alert">
              {error}
            </p>
          )}
          <div className="dialog-footer">
            <button className="button" disabled={!!busy} onClick={closeExport}>
              Fechar
            </button>
            <button
              className="button primary"
              disabled={
                !!busy ||
                !selected.size ||
                (mode === "json" && selected.size !== 1)
              }
              onClick={() => void exportData()}
            >
              {busy ? (
                <LoaderCircle size={15} className="spin" />
              ) : (
                <ArrowDownToLine size={15} />
              )}
              Escolher destino e exportar
            </button>
          </div>
        </Dialog>
      )}
      {record && (
        <Dialog title={`Medição ${record.id}`} close={() => setRecord(null)}>
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
              {s}
            </p>
          ))}
        </Dialog>
      )}
      {help && (
        <Dialog title="FC2 Explorer" close={() => setHelp(false)}>
          <p>
            Abra um FC2 e escolha o trabalho na lista lateral. As caixas de
            seleção definem os trabalhos para exportação.
          </p>
          <p>
            Use as abas para consultar medições, gráficos, coordenadas e
            metadados. Clique nos cabeçalhos da tabela para ordenar.
          </p>
          <p>
            <kbd>Ctrl O</kbd> Abrir arquivo · <kbd>Esc</kbd> Fechar diálogo
          </p>
          <p className="quiet-note">
            Leitura estrutural da variante FC2 documentada. Equivalência Leaf
            não presumida. Datum não confirmado.
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

function Chart({ rows }: { rows: Row[] }) {
  const [series, setSeries] = useState("chlorophyll_total_index");
  if (!rows.length)
    return (
      <div className="empty-state">
        <BarChart3 size={28} />
        <p>Nenhuma medição para visualizar.</p>
      </div>
    );
  const values = rows.map((r) => Number(r[series as keyof Measurement]));
  const max = Math.max(...values, 1) * 1.12;
  const width = 800,
    height = 260;
  return (
    <div className="chart-content">
      <div className="chart-heading">
        <div>
          <h3>Índices de clorofila</h3>
          <p>Índices armazenados, na ordem de origem dos registros.</p>
        </div>
        <select
          aria-label="Série do gráfico"
          value={series}
          onChange={(e) => setSeries(e.target.value)}
        >
          <option value="chlorophyll_total_index">Clorofila total</option>
          <option value="chlorophyll_a_index">Clorofila A</option>
          <option value="chlorophyll_b_index">Clorofila B</option>
        </select>
      </div>
      <svg
        className="data-chart"
        viewBox="0 0 870 330"
        role="img"
        aria-label={`Gráfico de ${values.length} medições de clorofila`}
      >
        {[0, 1, 2, 3, 4].map((i) => (
          <g key={i}>
            <line
              x1="48"
              x2="848"
              y1={20 + (height * i) / 4}
              y2={20 + (height * i) / 4}
              className="gridline"
            />
            <text x="37" y={24 + (height * i) / 4} textAnchor="end">
              {(max * (1 - i / 4)).toFixed(1).replace(".", ",")}
            </text>
          </g>
        ))}
        {values.map((v, i) => (
          <circle
            key={i}
            cx={
              48 + width * (values.length === 1 ? 0.5 : i / (values.length - 1))
            }
            cy={20 + height * (1 - v / max)}
            r={values.length > 500 ? 1.6 : 3}
            className="chart-point"
          >
            <title>
              Grupo {rows[i].group} · Medição {rows[i].id}: {value(v)}
            </title>
          </circle>
        ))}
        <text x="48" y="305">
          1
        </text>
        <text x="848" y="305" textAnchor="end">
          {values.length}
        </text>
        <text x="448" y="322" textAnchor="middle">
          Posição na sequência de origem
        </text>
      </svg>
      <div className="quiet-note">
        <Info size={16} />
        <p>
          Valores em índice do instrumento. Os pontos não representam médias nem
          réplicas experimentais.
        </p>
      </div>
    </div>
  );
}
function CoordinatePlot({ rows }: { rows: Row[] }) {
  const points = rows.filter((r) => r.latitude != null && r.longitude != null);
  if (!points.length)
    return (
      <div className="empty-state">
        <MapPin size={30} />
        <h3>Sem coordenadas disponíveis</h3>
        <p>As ausências são preservadas no arquivo de origem.</p>
      </div>
    );
  const lons = points.map((r) => r.longitude!),
    lats = points.map((r) => r.latitude!);
  const minX = Math.min(...lons),
    maxX = Math.max(...lons),
    minY = Math.min(...lats),
    maxY = Math.max(...lats);
  const spanX = maxX - minX || 0.001,
    spanY = maxY - minY || 0.001;
  const x = (v: number) =>
    maxX === minX ? 450 : 90 + ((v - minX) / spanX) * 720;
  const y = (v: number) =>
    maxY === minY ? 175 : 295 - ((v - minY) / spanY) * 240;
  return (
    <div className="chart-content">
      <div className="chart-heading">
        <div>
          <h3>Coordenadas dos registros</h3>
          <p>
            {number(points.length)} registros com coordenadas · visualização
            local
          </p>
        </div>
        <span className="badge neutral">
          <MapPin size={13} /> Sem mapa externo
        </span>
      </div>
      <svg
        className="coordinate-chart"
        viewBox="0 0 900 355"
        role="img"
        aria-label="Dispersão de longitude e latitude"
      >
        <rect
          x="70"
          y="30"
          width="760"
          height="285"
          rx="8"
          className="plot-background"
        />
        {[0, 1, 2, 3, 4].map((i) => (
          <g key={i}>
            <line
              x1="90"
              x2="810"
              y1={55 + i * 60}
              y2={55 + i * 60}
              className="gridline"
            />
            <text x="82" y={60 + i * 60} textAnchor="end">
              {(maxY - (i / 4) * (maxY - minY)).toFixed(3)}
            </text>
            <text x={90 + i * 180} y="334" textAnchor="middle">
              {(minX + (i / 4) * (maxX - minX)).toFixed(3)}
            </text>
          </g>
        ))}
        {points.map((r, i) => (
          <circle
            key={i}
            cx={x(r.longitude!)}
            cy={y(r.latitude!)}
            r="4"
            className="chart-point"
          >
            <title>
              Grupo {r.group} · Medição {r.id}: {r.latitude}, {r.longitude}
            </title>
          </circle>
        ))}
        <text x="450" y="353" textAnchor="middle">
          Longitude (°)
        </text>
        <text x="15" y="170" transform="rotate(-90 15 170)" textAnchor="middle">
          Latitude (°)
        </text>
      </svg>
      <div className="quiet-note">
        <Info size={16} />
        <p>
          Datum não confirmado. Os eixos são ajustados independentemente; este
          gráfico não representa distâncias geográficas.
        </p>
      </div>
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
