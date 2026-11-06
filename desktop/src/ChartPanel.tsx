import { useMemo, useState } from "react";
import { BarChart3, Info, Minimize2, ZoomIn, ZoomOut } from "lucide-react";
import {
  chartRows,
  niceTicks,
  SERIES_NAMES,
  type ChartRecord,
  type ChartRow,
} from "./chartData";
import { useSize } from "./useSize";
import { Select } from "./Select";

const COLORS = ["#339e84", "#db9841", "#5e87d8"] as const;
const VARIABLES: Record<string, number[]> = {
  total: [2],
  a: [0],
  b: [1],
  all: [0, 1, 2],
};
const decimal = (n: number, digits = 1) =>
  n.toLocaleString("pt-BR", {
    minimumFractionDigits: 0,
    maximumFractionDigits: digits,
  });
const PLOT_H = 300;
const TOP = 14;
const LABEL_H = 78;

export function ChartPanel({ records }: { records: ChartRecord[] }) {
  const [mode, setMode] = useState<"mean" | "raw">("mean");
  const [variable, setVariable] = useState("total");
  const [zoom, setZoom] = useState(1);
  const [tip, setTip] = useState<{
    x: number;
    y: number;
    row: ChartRow;
    series: number;
  } | null>(null);
  const [box, size] = useSize<HTMLDivElement>();
  const data = useMemo(() => chartRows(records), [records]);
  const rows = mode === "raw" ? data.raw : data.means;
  const series = VARIABLES[variable];
  const multi = series.length === 3;
  if (!records.length)
    return (
      <div className="empty-state">
        <BarChart3 size={28} />
        <p>Este trabalho não possui leituras para representar.</p>
      </div>
    );
  const baseSlot = multi ? 72 : 48;
  const fitZoom = Math.min(
    8,
    Math.max(0.01, (size.width - 24) / (Math.max(rows.length, 1) * baseSlot)),
  );
  const slot = baseSlot * zoom;
  // Thin out category labels so they never overlap.
  const labelStep = Math.max(1, Math.ceil(15 / slot));
  const width = Math.max(rows.length * slot + 24, size.width);
  const max = Math.max(
    0,
    ...rows.flatMap((r) =>
      series.map(
        (s) => r.values[s] + (mode === "mean" ? (r.errors[s] ?? 0) : 0),
      ),
    ),
  );
  const { top, ticks } = niceTicks(max);
  const y = (v: number) => TOP + PLOT_H * (1 - v / top);
  const barW = (multi ? 0.24 : 0.65) * slot;
  const centre = (i: number) => 12 + (i + 0.5) * slot;
  const offset = (position: number) =>
    multi ? (position - 1) * 0.26 * slot : 0;
  return (
    <div className="chart-content chart-panel">
      <div className="chart-heading">
        <div>
          <h3>Índices de clorofila</h3>
          <p>
            {data.raw.length} leituras · {data.means.length} pontos por grupo ·
            índices instrumentais. Role horizontalmente para ver todas as
            barras.
          </p>
        </div>
        <div className="chart-controls">
          <div className="segmented" role="group" aria-label="Modo do gráfico">
            <button
              aria-pressed={mode === "mean"}
              onClick={() => setMode("mean")}
            >
              Média por ponto ± EP
            </button>
            <button
              aria-pressed={mode === "raw"}
              title="Mostrar todas as medidas individuais, identificadas pelo ID da leitura"
              onClick={() => setMode("raw")}
            >
              Todos os pontos
            </button>
          </div>
          <Select
            label="Série do gráfico"
            value={variable}
            onChange={setVariable}
            options={[
              { value: "total", label: "Clorofila total" },
              { value: "a", label: "Clorofila a" },
              { value: "b", label: "Clorofila b" },
              { value: "all", label: "Todas" },
            ]}
          />
          <button
            className="icon-button bordered"
            aria-label="Aproximar gráfico"
            onClick={() => setZoom((z) => Math.min(z * 1.25, 8))}
          >
            <ZoomIn size={15} />
          </button>
          <button
            className="icon-button bordered"
            aria-label="Afastar gráfico"
            onClick={() => setZoom((z) => Math.max(z * 0.8, 0.01))}
          >
            <ZoomOut size={15} />
          </button>
          <button
            className="icon-button bordered"
            aria-label="Ajustar à largura"
            title="Ajustar à largura: mostrar todas as barras sem rolagem"
            onClick={() => setZoom(fitZoom)}
          >
            <Minimize2 size={15} />
          </button>
        </div>
      </div>
      <div className="chart-legend" aria-label="Legenda">
        <span>Legenda:</span>
        {series.map((s) => (
          <span key={s} style={{ color: COLORS[s] }}>
            <i style={{ background: COLORS[s] }} />
            {SERIES_NAMES[s]}
          </span>
        ))}
      </div>
      <div className="chart-frame">
        <svg
          className="chart-axis"
          width="62"
          height={TOP + PLOT_H + 8}
          aria-hidden="true"
        >
          {ticks.map((t) => (
            <text key={t} x="54" y={y(t) + 3} textAnchor="end">
              {decimal(t)}
            </text>
          ))}
          <text
            x="12"
            y={TOP + PLOT_H / 2}
            transform={`rotate(-90 12 ${TOP + PLOT_H / 2})`}
            textAnchor="middle"
          >
            Índice instrumental
          </text>
        </svg>
        <div className="chart-scroll" ref={box}>
          <svg
            className="bar-chart"
            width={width}
            height={TOP + PLOT_H + LABEL_H}
            role="img"
            aria-label={`Gráfico de ${rows.length} ${mode === "raw" ? "leituras" : "pontos"} de clorofila`}
            onMouseLeave={() => setTip(null)}
          >
            {ticks.map((t) => (
              <line
                key={t}
                x1="0"
                x2={width}
                y1={y(t)}
                y2={y(t)}
                className={t === 0 ? "axis-line" : "gridline"}
              />
            ))}
            {rows.map((row, i) =>
              series.map((s, position) => {
                const x = centre(i) + offset(position);
                const h = Math.max(y(0) - y(row.values[s]), 0);
                const error = mode === "mean" ? row.errors[s] : null;
                return (
                  <g key={`${i}-${s}`}>
                    <rect
                      className="bar"
                      data-series={s}
                      x={x - barW / 2}
                      y={y(row.values[s])}
                      width={barW}
                      height={h}
                      fill={COLORS[s]}
                      onMouseMove={(e) =>
                        setTip({ x: e.clientX, y: e.clientY, row, series: s })
                      }
                    />
                    {error != null && (
                      <path
                        className="error-bar"
                        d={`M${x} ${y(row.values[s] - error)}V${y(row.values[s] + error)}M${x - barW * 0.3} ${y(row.values[s] - error)}H${x + barW * 0.3}M${x - barW * 0.3} ${y(row.values[s] + error)}H${x + barW * 0.3}`}
                      />
                    )}
                  </g>
                );
              }),
            )}
            {rows.map((row, i) =>
              i % labelStep ? null : (
                <text
                  key={i}
                  className="x-label"
                  transform={`translate(${centre(i) - 3} ${y(0) + 14}) rotate(45)`}
                >
                  {row.label}
                </text>
              ),
            )}
          </svg>
        </div>
        {tip && <ChartTip tip={tip} mode={mode} />}
      </div>
      <p className="chart-axis-title">
        {mode === "raw"
          ? "ID da leitura (categorias na ordem original)"
          : "Grupo · ponto (ordem da primeira leitura)"}
      </p>
      <div className="quiet-note">
        <Info size={16} />
        <p>
          {mode === "mean"
            ? "Média calculada por grupo e ponto. EP = desvio padrão amostral / √n; n = 1: EP indisponível. Leituras com avisos são mantidas. A clorofila total é o valor armazenado, não A + B."
            : "Valores armazenados em cada leitura, na ordem de origem. A clorofila total é o valor armazenado, não A + B."}{" "}
          Os filtros da tabela se aplicam ao gráfico.
        </p>
      </div>
    </div>
  );
}

function ChartTip({
  tip,
  mode,
}: {
  tip: { x: number; y: number; row: ChartRow; series: number };
  mode: "mean" | "raw";
}) {
  const { row, series } = tip;
  const left = Math.min(tip.x + 14, window.innerWidth - 350);
  const topPos = Math.min(tip.y + 14, window.innerHeight - 230);
  return (
    <div className="chart-tip" role="tooltip" style={{ left, top: topPos }}>
      <strong>{SERIES_NAMES[series]}</strong>
      {mode === "raw" && <div>ID {row.ids[0]}</div>}
      <div>
        Grupo {row.group} · ponto {row.point}
      </div>
      <small>{row.sourceFile}</small>
      <hr />
      {row.exact ? (
        <div>Valor: {row.exact[series].replace(".", ",")}</div>
      ) : (
        <>
          <div>Média: {row.values[series].toFixed(6).replace(".", ",")}</div>
          <div>
            Erro padrão:{" "}
            {row.errors[series] != null
              ? row.errors[series]!.toFixed(6).replace(".", ",")
              : "indisponível (n = 1)"}
          </div>
          <div>n: {row.ids.length} leituras</div>
          <div className="tip-ids">IDs das leituras: {row.ids.join(", ")}</div>
        </>
      )}
      <small>Avisos nas leituras: {row.warnings}</small>
    </div>
  );
}
