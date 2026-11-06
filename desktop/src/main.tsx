import React, { useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import {
  ArrowDownToLine,
  ArrowLeft,
  ArrowRight,
  BarChart3,
  Check,
  CheckCheck,
  ChevronLeft,
  ChevronRight,
  CircleHelp,
  Database,
  FileCheck2,
  FileJson2,
  FileSpreadsheet,
  FileUp,
  FolderOpen,
  HardDrive,
  Info,
  Leaf,
  LoaderCircle,
  MapPin,
  Moon,
  Search,
  ShieldCheck,
  SlidersHorizontal,
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

const steps = [
  {
    title: "Arquivo de origem",
    description: "Abra seus dados",
    icon: FolderOpen,
  },
  { title: "Trabalhos", description: "Escolha o que explorar", icon: Database },
  {
    title: "Explorar dados",
    description: "Conheça suas medições",
    icon: BarChart3,
  },
  {
    title: "Exportar",
    description: "Leve os dados com você",
    icon: ArrowDownToLine,
  },
];
const formats = [
  {
    id: "simple",
    title: "Planilhas Excel",
    tag: "Recomendado",
    description:
      "Uma planilha XLSX por trabalho, com medições e informações da origem.",
    icon: FileSpreadsheet,
  },
  {
    id: "complete",
    title: "Pacote completo",
    tag: "Com auditoria",
    description:
      "Planilhas, dados de auditoria e arquivos de origem da seleção.",
    icon: FileCheck2,
  },
  {
    id: "json",
    title: "Auditoria JSON",
    tag: "Um trabalho",
    description: "Registros, bytes e rastreabilidade de um único trabalho.",
    icon: FileJson2,
  },
];
const number = (n: number) => n.toLocaleString("pt-BR");
const date = (s: string | null) => (s ? s.split("-").reverse().join("/") : "—");
const value = (s: string | number | null | undefined) =>
  s == null ? "—" : String(s).replace(".", ",");
const PAGE_SIZE = 75;
type Row = Measurement & { group: number; source_file: string; index: number };

function App() {
  const [step, setStep] = useState(0);
  const [source, setSource] = useState<SourceSummary | null>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [active, setActive] = useState<number | null>(null);
  const [detail, setDetail] = useState<WorkDetail | null>(null);
  const [detailError, setDetailError] = useState("");
  const [detailAttempt, setDetailAttempt] = useState(0);
  const [tab, setTab] = useState("table");
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(0);
  const [group, setGroup] = useState("all");
  const [mode, setMode] = useState("simple");
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [result, setResult] = useState<ExportResult | null>(null);
  const [record, setRecord] = useState<Row | null>(null);
  const [help, setHelp] = useState(false);
  const [drag, setDrag] = useState(false);
  const [theme, setTheme] = useState(
    () => localStorage.getItem("fc2-theme") || "light",
  );
  const request = useRef(0);
  const title = useRef<HTMLHeadingElement>(null);
  const selectedWorks = source?.works.filter((w) => selected.has(w.key)) || [];
  const selectedRecords = selectedWorks.reduce((sum, w) => sum + w.records, 0);
  const rows: Row[] =
    detail?.groups.flatMap((g) =>
      g.records.map((r, index) => ({
        ...r,
        group: g.group,
        source_file: g.source_file,
        index,
      })),
    ) || [];
  const filteredRows = rows.filter(
    (r) => group === "all" || String(r.group) === group,
  );
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
  useEffect(() => {
    title.current?.focus({ preventScroll: true });
    title.current?.closest(".workspace")?.scrollTo({ top: 0 });
  }, [step]);

  const open = useCallback(
    async (path?: string) => {
      if (busy) return;
      if (!isTauri()) {
        setError(
          "Abra o aplicativo desktop para selecionar arquivos locais. Esta é uma prévia da interface.",
        );
        return;
      }
      setBusy("Lendo o arquivo e verificando sua estrutura…");
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
          setStep(1);
          setPage(0);
          setGroup("all");
        }
      } catch (e) {
        setError(String(e));
      } finally {
        setBusy("");
      }
    },
    [busy],
  );

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
  }, []);
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
  function navigate(next: number) {
    if (busy) return;
    setStep(next);
    setError("");
  }
  async function exportData() {
    if (!source || !selected.size || busy) return;
    setBusy("Validando a origem e exportando os trabalhos…");
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
      setBusy("");
    }
  }
  const canContinue = !!source && (step === 0 || selected.size > 0);
  const activeWork = source?.works.find((w) => w.key === active);

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand">
          <span className="brand-icon">
            <Leaf size={22} />
          </span>
          <span>
            FC2 <strong>Explorer</strong>
          </span>
          <span className="desktop-label">DESKTOP</span>
        </div>
        <div className="header-actions">
          <span className="local-label">
            <span className="status-dot" /> Processamento local
          </span>
          <button
            className="icon-button"
            aria-label={
              theme === "light" ? "Ativar tema escuro" : "Ativar tema claro"
            }
            onClick={() => setTheme(theme === "light" ? "dark" : "light")}
          >
            {theme === "light" ? <Moon size={18} /> : <Sun size={18} />}
          </button>
          <button
            className="icon-button"
            aria-label="Ajuda"
            onClick={() => setHelp(true)}
          >
            <CircleHelp size={19} />
          </button>
        </div>
      </header>
      <aside className="sidebar">
        <div className="sidebar-heading">SEU ESPAÇO DE ANÁLISE</div>
        <nav aria-label="Etapas">
          {steps.map((s, i) => (
            <button
              key={s.title}
              className={`step ${step === i ? "active" : ""}`}
              aria-current={step === i ? "step" : undefined}
              disabled={
                !!busy || (i > 0 && !source) || (i > 1 && !selected.size)
              }
              onClick={() => navigate(i)}
            >
              <span className={`step-number ${i < step ? "done" : ""}`}>
                {i < step ? <Check size={16} /> : `0${i + 1}`}
              </span>
              <span>
                <strong>{s.title}</strong>
                <small>{s.description}</small>
              </span>
              {step === i && <span className="active-dot" />}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="orbit orbit-one" />
          <div className="orbit orbit-two" />
          <Leaf size={27} />
          <p>
            Da medição
            <br />
            <strong>à descoberta.</strong>
          </p>
          <span>Seus dados, mais claros.</span>
        </div>
        <div className="sidebar-version">
          FC2 Explorer <span>v0.2.0</span>
        </div>
      </aside>
      <main className="main">
        <div className="workspace" aria-busy={!!busy}>
          <div className="page-heading">
            <div>
              <div className="eyebrow">
                ETAPA 0{step + 1} DE 04 <span />{" "}
                {step === 0 ? "COMECE POR AQUI" : source?.name}
              </div>
              <h1 ref={title} tabIndex={-1}>
                {
                  [
                    "Seus dados começam aqui.",
                    "Quais trabalhos vamos explorar?",
                    "Um olhar mais próximo.",
                    "Pronto para o próximo passo.",
                  ][step]
                }
              </h1>
              <p>
                {
                  [
                    "Abra um arquivo FC2 para organizar, explorar e exportar suas medições.",
                    "Selecione os trabalhos que deseja analisar ou exportar.",
                    "Explore as medições e confira as informações de cada trabalho.",
                    "Escolha como levar suas medições para fora do FC2 Explorer.",
                  ][step]
                }
              </p>
            </div>
            <span className="page-icon">
              {React.createElement(steps[step].icon, {
                size: 25,
                strokeWidth: 1.5,
              })}
            </span>
          </div>
          {error && (
            <div className="notice error" role="alert">
              <TriangleAlert size={19} />
              <span>{error}</span>
              <button
                className="icon-button"
                aria-label="Fechar aviso"
                onClick={() => setError("")}
              >
                <X size={16} />
              </button>
            </div>
          )}
          {busy && (
            <div className="notice loading" role="status">
              <LoaderCircle className="spin" size={19} />
              {busy}
            </div>
          )}
          {step === 0 && (
            <>
              <section className={`import-card ${drag ? "dragging" : ""}`}>
                <div className="import-copy">
                  <div className="file-symbol">
                    <FileUp size={30} strokeWidth={1.5} />
                    <span>.FC2</span>
                  </div>
                  <h2>Abra uma nova perspectiva.</h2>
                  <p>
                    Seu arquivo de medições, em um espaço
                    <br className="desktop-break" /> pensado para explorar cada
                    detalhe.
                  </p>
                  <button
                    className="button primary large"
                    disabled={!!busy}
                    onClick={() => void open()}
                  >
                    <FolderOpen size={18} /> Selecionar arquivo FC2
                  </button>
                  <span className="drop-hint">
                    ou arraste um arquivo para esta janela <kbd>Ctrl O</kbd>
                  </span>
                  {source && (
                    <button className="text-button" onClick={() => navigate(1)}>
                      Continuar com {source.name} <ArrowRight size={15} />
                    </button>
                  )}
                </div>
                <div className="import-visual" aria-hidden="true">
                  <div className="visual-orbit" />
                  <div className="visual-orbit second" />
                  <div className="visual-dots" />
                  <div className="specimen-card">
                    <div className="specimen-top">
                      <span className="specimen-icon">
                        <Leaf size={21} />
                      </span>
                      <span>
                        Clorofila
                        <span className="specimen-caption">
                          UMA NOVA VISÃO DOS SEUS DADOS
                        </span>
                      </span>
                      <span className="specimen-check">
                        <Check size={14} />
                      </span>
                    </div>
                    <div className="specimen-bars">
                      {[37, 55, 48, 72, 59, 81, 70, 91, 77, 95, 83, 100].map(
                        (h, i) => (
                          <span
                            key={i}
                            style={{
                              height: `${h}%`,
                              opacity: 0.3 + i * 0.055,
                            }}
                          />
                        ),
                      )}
                    </div>
                    <div className="specimen-bottom">
                      <span>COLETAR</span>
                      <span>EXPLORAR</span>
                      <span>DESCOBRIR</span>
                    </div>
                  </div>
                  <div className="floating-label">
                    <CheckCheck size={16} /> Cada medição conta.
                  </div>
                </div>
              </section>
              <div className="feature-grid">
                <Feature
                  icon={HardDrive}
                  title="Do arquivo ao insight"
                  text="Trabalhos e medições organizados em um só lugar."
                />
                <Feature
                  icon={ShieldCheck}
                  title="Seus dados ficam com você"
                  text="Leitura e exportação locais, sem upload ou conta."
                />
                <Feature
                  icon={FileSpreadsheet}
                  title="Pronto para seguir adiante"
                  text="Exporte para Excel ou preserve a auditoria completa."
                />
              </div>
              <div className="quiet-note">
                <Info size={16} />
                <p>
                  Compatível com a variante FC2 documentada pelo projeto. O
                  arquivo original é preservado.
                </p>
              </div>
            </>
          )}
          {step === 1 && source && (
            <>
              <div className="source-strip">
                <div className="source-file-icon">
                  <FileCheck2 size={23} />
                </div>
                <div>
                  <strong>{source.name}</strong>
                  <small>
                    {(source.size / 1024 / 1024).toLocaleString("pt-BR", {
                      maximumFractionDigits: 2,
                    })}{" "}
                    MB <span>·</span> {number(source.works.length)} trabalhos
                    encontrados
                  </small>
                </div>
                <button
                  className="button subtle"
                  disabled={!!busy}
                  onClick={() => void open()}
                >
                  <FolderOpen size={16} /> Trocar arquivo
                </button>
              </div>
              <div className="stats-grid">
                <Stat label="Trabalhos" value={source.works.length} />
                <Stat label="Medições" value={source.records} />
                <Stat label="Grupos" value={source.groups} />
                <Stat label="Com coordenadas" value={source.coordinates} />
              </div>
              <section className="panel works-panel">
                <div className="panel-toolbar">
                  <label className="search">
                    <Search size={18} />
                    <input
                      aria-label="Buscar trabalhos"
                      placeholder="Buscar por nome ou número…"
                      value={search}
                      onChange={(e) => setSearch(e.target.value)}
                    />
                    {search && (
                      <button
                        className="icon-button"
                        aria-label="Limpar busca"
                        onClick={() => setSearch("")}
                      >
                        <X size={14} />
                      </button>
                    )}
                  </label>
                  <span className="selection-count">
                    {selected.size} selecionados
                  </span>
                  <button
                    className="text-button"
                    onClick={() => {
                      setSelected(new Set(source.works.map((w) => w.key)));
                      setResult(null);
                    }}
                  >
                    Selecionar todos
                  </button>
                  <button
                    className="text-button muted"
                    onClick={() => {
                      setSelected(new Set());
                      setResult(null);
                    }}
                  >
                    Limpar
                  </button>
                </div>
                <div className="table-scroll">
                  <table className="works-table">
                    <thead>
                      <tr>
                        <th className="check-cell">
                          <input
                            aria-label="Selecionar trabalhos visíveis"
                            type="checkbox"
                            checked={
                              works.length > 0 &&
                              works.every((w) => selected.has(w.key))
                            }
                            onChange={(e) => {
                              const next = new Set(selected);
                              works.forEach((w) =>
                                e.target.checked
                                  ? next.add(w.key)
                                  : next.delete(w.key),
                              );
                              setSelected(next);
                              setResult(null);
                            }}
                          />
                        </th>
                        <th>Trabalho</th>
                        <th>Medições</th>
                        <th>Grupos</th>
                        <th>Período</th>
                        <th>Situação</th>
                        <th />
                      </tr>
                    </thead>
                    <tbody>
                      {works.map((w) => (
                        <tr
                          key={w.key}
                          className={selected.has(w.key) ? "selected" : ""}
                        >
                          <td>
                            <input
                              aria-label={`Selecionar ${w.name}`}
                              type="checkbox"
                              checked={selected.has(w.key)}
                              onChange={() => toggle(w.key)}
                            />
                          </td>
                          <td>
                            <strong>{w.name}</strong>
                            <small className="table-secondary">
                              Trabalho {w.number} · {number(w.points)} pontos
                            </small>
                          </td>
                          <td>{number(w.records)}</td>
                          <td>{w.groups}</td>
                          <td className="date-cell">
                            {date(w.first_date)}
                            {w.last_date && w.last_date !== w.first_date && (
                              <small className="table-secondary">
                                a {date(w.last_date)}
                              </small>
                            )}
                          </td>
                          <td>
                            <span
                              className={`badge ${w.errors.length ? "danger" : w.warnings ? "warning" : "success"}`}
                            >
                              {w.errors.length ? (
                                <TriangleAlert size={13} />
                              ) : w.warnings ? (
                                <Info size={13} />
                              ) : (
                                <Check size={13} />
                              )}{" "}
                              {w.errors.length
                                ? "Verificar"
                                : w.warnings
                                  ? `${w.warnings} avisos`
                                  : "Lido"}
                            </span>
                          </td>
                          <td>
                            <button
                              className="icon-button"
                              aria-label={`Explorar ${w.name}`}
                              onClick={() => {
                                setActive(w.key);
                                if (!selected.has(w.key)) toggle(w.key);
                                navigate(2);
                              }}
                            >
                              <ChevronRight size={18} />
                            </button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                {!works.length && (
                  <div className="empty-state">
                    <Search size={28} />
                    <h3>
                      {source.works.length
                        ? "Nenhum trabalho encontrado"
                        : "Nenhum trabalho ativo encontrado"}
                    </h3>
                    <p>
                      {source.works.length
                        ? "Tente outro nome ou número."
                        : "Abra outro arquivo FC2 para continuar."}
                    </p>
                  </div>
                )}
                <div className="table-bottom">
                  {number(works.length)} trabalhos{" "}
                  <span>{number(selectedRecords)} medições na seleção</span>
                </div>
              </section>
            </>
          )}
          {step === 2 && source && (
            <>
              <div className="explore-toolbar">
                <label className="field-label">
                  Trabalho
                  <select
                    aria-label="Trabalho em exploração"
                    value={active ?? ""}
                    onChange={(e) => setActive(Number(e.target.value))}
                  >
                    {source.works.map((w) => (
                      <option value={w.key} key={w.key}>
                        {w.name} · {w.number}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="field-label">
                  Grupo
                  <select
                    aria-label="Filtrar grupo"
                    value={group}
                    onChange={(e) => {
                      setGroup(e.target.value);
                      setPage(0);
                    }}
                  >
                    {<option value="all">Todos os grupos</option>}
                    {detail?.groups.map((g) => (
                      <option key={g.group} value={g.group}>
                        Grupo {g.group}
                      </option>
                    ))}
                  </select>
                </label>
                {activeWork && (
                  <span className="explore-meta">
                    {number(activeWork.records)} medições <span>·</span>{" "}
                    {number(activeWork.points)} pontos
                  </span>
                )}
              </div>
              <section className="panel">
                <div
                  className="tabs"
                  role="tablist"
                  aria-label="Visualização dos dados"
                >
                  {[
                    { id: "table", name: "Medições", icon: Table2 },
                    { id: "chart", name: "Gráfico", icon: BarChart3 },
                    { id: "map", name: "Coordenadas", icon: MapPin },
                    {
                      id: "source",
                      name: "Origem e avisos",
                      icon: ShieldCheck,
                    },
                  ].map((t) => (
                    <button
                      key={t.id}
                      role="tab"
                      aria-selected={tab === t.id}
                      className={tab === t.id ? "active" : ""}
                      onClick={() => setTab(t.id)}
                    >
                      <t.icon size={16} />
                      {t.name}
                    </button>
                  ))}
                </div>
                {detailError ? (
                  <div className="empty-state" role="alert">
                    <TriangleAlert size={25} />
                    <h3>Não foi possível carregar o trabalho</h3>
                    <p>{detailError}</p>
                    <button
                      className="button secondary"
                      onClick={() => setDetailAttempt((n) => n + 1)}
                    >
                      Tentar novamente
                    </button>
                  </div>
                ) : !detail ? (
                  <div className="empty-state" role="status">
                    <LoaderCircle className="spin" size={25} />
                    <p>Carregando medições…</p>
                  </div>
                ) : (
                  <div role="tabpanel">
                    {tab === "table" && (
                      <>
                        <div className="data-caption">
                          <span>
                            Índices de clorofila armazenados no instrumento
                          </span>
                          <span>
                            Clique em uma medição para inspecionar a origem
                          </span>
                        </div>
                        <div className="table-scroll">
                          <table className="measure-table">
                            <thead>
                              <tr>
                                {[
                                  "Grupo",
                                  "Medição",
                                  "Ponto",
                                  "Data · Hora",
                                  "Clorofila A",
                                  "Clorofila B",
                                  "Total",
                                  "",
                                ].map((s, i) => (
                                  <th key={i}>{s}</th>
                                ))}
                              </tr>
                            </thead>
                            <tbody>
                              {filteredRows
                                .slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE)
                                .map((r) => (
                                  <tr key={`${r.group}-${r.index}`}>
                                    <td>
                                      <span className="group-pill">
                                        {r.group}
                                      </span>
                                    </td>
                                    <td>{r.id}</td>
                                    <td>{r.point}</td>
                                    <td>
                                      {date(r.date)}
                                      <small className="table-secondary">
                                        {r.time || "—"}
                                      </small>
                                    </td>
                                    <td>{value(r.chlorophyll_a_index)}</td>
                                    <td>{value(r.chlorophyll_b_index)}</td>
                                    <td>
                                      <strong className="total-value">
                                        {value(r.chlorophyll_total_index)}
                                      </strong>
                                    </td>
                                    <td>
                                      <button
                                        className="icon-button"
                                        aria-label={`Inspecionar medição ${r.id} do grupo ${r.group}`}
                                        onClick={() => setRecord(r)}
                                      >
                                        {r.warnings.length ? (
                                          <TriangleAlert size={16} />
                                        ) : (
                                          <ChevronRight size={16} />
                                        )}
                                      </button>
                                    </td>
                                  </tr>
                                ))}
                            </tbody>
                          </table>
                        </div>
                        {!filteredRows.length && (
                          <div className="empty-state">
                            <Table2 size={26} />
                            <p>Este grupo não contém medições.</p>
                          </div>
                        )}
                        <div className="table-bottom">
                          {number(filteredRows.length)} medições{" "}
                          <div className="pagination">
                            <button
                              className="icon-button"
                              aria-label="Página anterior"
                              disabled={page === 0}
                              onClick={() => setPage(page - 1)}
                            >
                              <ChevronLeft size={17} />
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
                              <ChevronRight size={17} />
                            </button>
                          </div>
                        </div>
                      </>
                    )}
                    {tab === "chart" && <Chart rows={filteredRows} />}
                    {tab === "map" && <CoordinatePlot rows={filteredRows} />}
                    {tab === "source" && (
                      <div className="audit-content">
                        <h3>Um caminho de volta à origem.</h3>
                        <p>
                          As medições mantêm seus valores e ausências conforme a
                          leitura do arquivo.
                        </p>
                        <dl className="metadata">
                          <dt>Arquivo</dt>
                          <dd>{source.name}</dd>
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
                        {[...detail.errors, ...detail.warnings].map((w, i) => (
                          <div className="notice warning" key={i}>
                            <TriangleAlert size={17} />
                            <span>{w}</span>
                          </div>
                        ))}
                        <h4>Observações da leitura</h4>
                        {source.limitations.map((s, i) => (
                          <p className="limitation" key={i}>
                            <Info size={15} />
                            {s}
                          </p>
                        ))}
                        <div className="quiet-note">
                          <ShieldCheck size={18} />
                          <p>
                            A leitura estrutural não certifica equivalência
                            científica com uma referência Leaf. O datum das
                            coordenadas não está confirmado.
                          </p>
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </section>
            </>
          )}
          {step === 3 && source && (
            <>
              <div className="export-layout">
                <div>
                  <div className="section-label">FORMATO DE SAÍDA</div>
                  <div
                    className="format-options"
                    role="radiogroup"
                    aria-label="Formato de exportação"
                  >
                    {formats.map((f) => (
                      <label
                        className={`format-card ${mode === f.id ? "chosen" : ""}`}
                        key={f.id}
                      >
                        <input
                          type="radio"
                          name="format"
                          value={f.id}
                          checked={mode === f.id}
                          onChange={() => {
                            setMode(f.id);
                            setResult(null);
                          }}
                        />
                        <span className="format-icon">
                          <f.icon size={25} strokeWidth={1.5} />
                        </span>
                        <span className="format-copy">
                          <strong>{f.title}</strong>
                          <small>{f.description}</small>
                          <span className="format-tag">{f.tag}</span>
                        </span>
                        <span className="radio-circle">
                          {mode === f.id && <span />}
                        </span>
                      </label>
                    ))}
                  </div>
                  {mode === "json" && selected.size !== 1 && (
                    <div className="notice warning">
                      <Info size={18} />
                      <span>
                        Para JSON de auditoria, volte à seleção e escolha um
                        único trabalho.
                      </span>
                    </div>
                  )}
                </div>
                <aside className="export-summary">
                  <div className="summary-icon">
                    <FileCheck2 size={24} />
                  </div>
                  <h3>Tudo pronto?</h3>
                  <p>Confira o que você está levando.</p>
                  <dl>
                    <dt>Arquivo de origem</dt>
                    <dd>{source.name}</dd>
                    <dt>Trabalhos selecionados</dt>
                    <dd>{number(selected.size)}</dd>
                    <dt>Medições</dt>
                    <dd>{number(selectedRecords)}</dd>
                  </dl>
                  <div className="summary-divider" />
                  <div className="summary-note">
                    <ShieldCheck size={18} />
                    <span>
                      {mode === "json"
                        ? "Escolha onde salvar a auditoria JSON. Arquivos existentes são preservados."
                        : "Uma nova pasta será criada no destino escolhido. Os arquivos existentes são preservados."}
                    </span>
                  </div>
                  <button className="text-button" onClick={() => navigate(1)}>
                    <SlidersHorizontal size={15} /> Revisar seleção
                  </button>
                </aside>
              </div>
              {result && (
                <div
                  className={`export-result ${result.failures ? "partial" : ""}`}
                  role="status"
                >
                  <span className="result-icon">
                    {result.failures ? (
                      <TriangleAlert size={25} />
                    ) : (
                      <Check size={27} />
                    )}
                  </span>
                  <div>
                    <h3>
                      {result.failures
                        ? "Exportação concluída com pendências"
                        : "Seus dados estão prontos."}
                    </h3>
                    <p>
                      {mode === "json"
                        ? "Auditoria JSON salva."
                        : `${number(result.workbooks)} planilhas exportadas. ${result.failures ? `${result.failures} trabalhos bloqueados.` : "Você já pode continuar sua análise."}`}
                    </p>
                    <code>{result.path}</code>
                    {result.errors.map((e, i) => (
                      <p key={i}>{e}</p>
                    ))}
                  </div>
                </div>
              )}
              <div className="quiet-note">
                <Info size={16} />
                <p>
                  Campos ausentes ficam vazios. O total de clorofila é
                  preservado como armazenado no instrumento.
                </p>
              </div>
            </>
          )}
        </div>
        <footer className="footer">
          <div className="footer-status">
            <span className="footer-step">
              0{step + 1}
              <span> / 04</span>
            </span>
            <span>
              {source
                ? `${selected.size} ${selected.size === 1 ? "trabalho selecionado" : "trabalhos selecionados"}`
                : "Tudo começa com um arquivo."}
            </span>
          </div>
          <div className="footer-actions">
            {step > 0 && (
              <button
                className="button secondary"
                disabled={!!busy}
                onClick={() => navigate(step - 1)}
              >
                <ArrowLeft size={16} /> Voltar
              </button>
            )}
            {step < 3 ? (
              <button
                className="button primary"
                disabled={!canContinue || !!busy}
                onClick={() => navigate(step + 1)}
              >
                {step === 2 ? "Preparar exportação" : "Continuar"}
                <ArrowRight size={16} />
              </button>
            ) : (
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
                  <LoaderCircle className="spin" size={17} />
                ) : (
                  <ArrowDownToLine size={17} />
                )}{" "}
                Escolher destino e exportar
              </button>
            )}
          </div>
        </footer>
      </main>
      {drag && (
        <div className="drop-overlay">
          <FileUp size={50} />
          <h2>Solte seu arquivo FC2 aqui</h2>
        </div>
      )}
      {record && (
        <Dialog title={`Medição ${record.id}`} close={() => setRecord(null)}>
          <p className="dialog-subtitle">
            Grupo {record.group} · Ponto {record.point}
          </p>
          <dl className="metadata">
            <dt>Data e hora</dt>
            <dd>
              {date(record.date)} {record.time || "—"}
            </dd>
            <dt>Coordenadas</dt>
            <dd>
              {value(record.latitude)}, {value(record.longitude)}
            </dd>
            <dt>Arquivo de origem</dt>
            <dd>{record.source_file}</dd>
            <dt>SHA-256 do registro</dt>
            <dd className="mono">{record.raw_sha256}</dd>
          </dl>
          <h4>Segmentos no arquivo FC2</h4>
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
          {record.warnings.map((w, i) => (
            <div key={i} className="notice warning">
              <TriangleAlert size={16} />
              <span>{w}</span>
            </div>
          ))}
        </Dialog>
      )}
      {help && (
        <Dialog title="Um espaço para seus dados." close={() => setHelp(false)}>
          <p>
            O FC2 Explorer lê trabalhos ativos em arquivos Falker FC2 e organiza
            suas medições para exploração e exportação.
          </p>
          <div className="help-steps">
            {steps.map((s, i) => (
              <div key={s.title}>
                <span className="step-number">0{i + 1}</span>
                <span>
                  <strong>{s.title}</strong>
                  <p>{s.description}</p>
                </span>
              </div>
            ))}
          </div>
          <p>
            <kbd>Ctrl O</kbd> Abrir arquivo <span className="help-gap" />
            <kbd>Esc</kbd> Fechar diálogo
          </p>
          <div className="quiet-note">
            <Info size={17} />
            <p>
              Ferramenta independente, sem vínculo com a Falker. Compatibilidade
              limitada à variante documentada pelo projeto.
            </p>
          </div>
        </Dialog>
      )}
    </div>
  );
}

function Feature({
  icon: Icon,
  title,
  text,
}: {
  icon: typeof Leaf;
  title: string;
  text: string;
}) {
  return (
    <div className="feature">
      <Icon size={21} strokeWidth={1.5} />
      <h3>{title}</h3>
      <p>{text}</p>
    </div>
  );
}
function Stat({ label, value: n }: { label: string; value: number }) {
  return (
    <div className="stat">
      <span>{label}</span>
      <strong>{number(n)}</strong>
    </div>
  );
}
function Dialog({
  title,
  close,
  children,
}: {
  title: string;
  close: () => void;
  children: React.ReactNode;
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
        <div className="dialog-footer">
          <button className="button primary" onClick={close}>
            Entendi
          </button>
        </div>
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
          <h3>Uma medição de cada vez.</h3>
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
          <h3>Onde as medições aconteceram.</h3>
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
