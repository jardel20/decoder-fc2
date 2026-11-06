import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Info, Maximize2, MapPin, Minus, Plus } from "lucide-react";
import {
  TILE,
  fitBounds,
  mercatorX,
  mercatorY,
  renderable,
  unmercatorX,
  unmercatorY,
  type MapPoint,
  type View,
} from "./geo";
import { useSize } from "./useSize";
import { loadTile } from "./tileLoader";
import { OfflineRegion } from "./OfflineRegion";
import { warningMessage } from "./warnings";

const MAX_ZOOM = 19;
const wrap = (lon: number) => ((((lon + 180) % 360) + 360) % 360) - 180;
const unit = (n: number) =>
  n.toLocaleString("pt-BR", { maximumFractionDigits: 1 });

export interface MapFocus {
  id: number;
  points: MapPoint[];
}
interface Props {
  title: string;
  points: MapPoint[];
  selectedKey: string | null;
  /** Work whose points are drawn larger and highlighted. */
  relatedWork: number | null;
  onSelect: (point: MapPoint) => void;
  /** Frame these points once; cleared through `onFocused` when handled. */
  focus: MapFocus | null;
  onFocused: () => void;
  fitKey: string;
  fill?: boolean;
  action?: React.ReactNode;
  /** Points of the whole file, used by the offline download. */
  region: MapPoint[];
}

export function MapPanel({
  title,
  points,
  selectedKey,
  relatedWork,
  onSelect,
  focus,
  onFocused,
  fitKey,
  fill,
  action,
  region,
}: Props) {
  const [box, size] = useSize<HTMLDivElement>();
  const [view, setView] = useState<View>({ lon: 0, lat: 0, zoom: 2 });
  const [hover, setHover] = useState<{
    x: number;
    y: number;
    hits: MapPoint[];
    index: number;
  } | null>(null);
  const [tileError, setTileError] = useState(false);
  const tileFailed = useCallback(() => setTileError(true), []);
  const drag = useRef<{ x: number; y: number; moved: number } | null>(null);
  const drawable = useMemo(() => points.filter(renderable), [points]);
  const unsupported = points.length - drawable.length;
  const world = TILE * 2 ** view.zoom;
  const cx = mercatorX(view.lon) * world;
  const cy = mercatorY(view.lat) * world;
  const project = (lon: number, lat: number) => ({
    x: mercatorX(lon) * world - cx + size.width / 2,
    y: mercatorY(lat) * world - cy + size.height / 2,
  });

  const fit = useCallback(() => {
    const next = fitBounds(drawable, size.width, size.height);
    if (next) setView(next);
  }, [drawable, size.width, size.height]);
  // Refit once per work/filter change, as soon as the viewport has a size.
  const fitted = useRef("");
  useEffect(() => {
    if (!size.width || fitted.current === fitKey) return;
    fitted.current = fitKey;
    fit();
  }, [fitKey, fit, size.width]);
  useEffect(() => {
    if (!focus) return;
    const next = fitBounds(
      focus.points.filter(renderable),
      size.width || 800,
      size.height || 500,
    );
    if (next) {
      fitted.current = fitKey;
      setView(next);
    }
    onFocused();
  }, [focus, fitKey, onFocused, size.width, size.height]);

  const zoomAt = useCallback(
    (delta: number, px: number, py: number) =>
      setView((old) => {
        const zoom = Math.min(MAX_ZOOM, Math.max(0, old.zoom + delta));
        if (zoom === old.zoom) return old;
        const oldWorld = TILE * 2 ** old.zoom;
        const newWorld = TILE * 2 ** zoom;
        // Keep the geographic location under the pointer fixed.
        const wx = mercatorX(old.lon) * oldWorld + (px - size.width / 2);
        const wy = mercatorY(old.lat) * oldWorld + (py - size.height / 2);
        const nx = (wx / oldWorld) * newWorld - (px - size.width / 2);
        const ny = (wy / oldWorld) * newWorld - (py - size.height / 2);
        return {
          zoom,
          lon: wrap(unmercatorX(nx / newWorld)),
          lat: unmercatorY(Math.min(1, Math.max(0, ny / newWorld))),
        };
      }),
    [size.width, size.height],
  );
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const wheel = (e: WheelEvent) => {
      e.preventDefault();
      const rect = el.getBoundingClientRect();
      zoomAt(-e.deltaY * 0.0025, e.clientX - rect.left, e.clientY - rect.top);
    };
    el.addEventListener("wheel", wheel, { passive: false });
    return () => el.removeEventListener("wheel", wheel);
  }, [box, zoomAt]);

  const tiles = useMemo(() => {
    if (!size.width) return [];
    const z = Math.min(MAX_ZOOM, Math.max(0, Math.round(view.zoom)));
    const scale = TILE * 2 ** (view.zoom - z);
    const n = 2 ** z;
    const left = (cx - size.width / 2) / scale;
    const top = (cy - size.height / 2) / scale;
    const out: { key: string; src: string; x: number; y: number; s: number }[] =
      [];
    for (
      let ty = Math.max(0, Math.floor(top));
      ty <= Math.min(n - 1, Math.floor(top + size.height / scale));
      ty++
    )
      for (
        let tx = Math.floor(left);
        tx <= Math.floor(left + size.width / scale);
        tx++
      )
        out.push({
          key: `${z}/${tx}/${ty}`,
          src: `${z}/${((tx % n) + n) % n}/${ty}`,
          x: tx * scale - (cx - size.width / 2),
          y: ty * scale - (cy - size.height / 2),
          s: scale,
        });
    return out.length > 150 ? [] : out;
  }, [view.zoom, cx, cy, size.width, size.height]);

  function nearest(px: number, py: number) {
    const hits: { d: number; point: MapPoint }[] = [];
    for (const p of drawable) {
      const s = project(p.lon, p.lat);
      const d = (s.x - px) ** 2 + (s.y - py) ** 2;
      if (d <= 100) hits.push({ d, point: p });
    }
    hits.sort((a, b) => a.d - b.d || (a.point.key < b.point.key ? -1 : 1));
    return hits.map((h) => h.point);
  }
  function local(e: React.PointerEvent | React.MouseEvent) {
    const rect = box.current!.getBoundingClientRect();
    return { x: e.clientX - rect.left, y: e.clientY - rect.top };
  }
  const shown = hover?.hits[hover.index] ?? null;
  const selected = drawable.find((p) => p.key === selectedKey) ?? null;
  // Selected on top, then the highlighted work, then everything else.
  const ordered = useMemo(
    () => [
      ...drawable.filter((p) => p.work !== relatedWork),
      ...drawable.filter((p) => p.work === relatedWork),
    ],
    [drawable, relatedWork],
  );
  return (
    <div className={`map-panel ${fill ? "fill" : ""}`}>
      <div className="chart-heading">
        <div>
          <h3>{title}</h3>
          <p>
            {drawable.length.toLocaleString("pt-BR")} coordenadas • zoom{" "}
            {unit(view.zoom)} • arraste e use a roda do mouse
          </p>
        </div>
        <div className="chart-controls">
          {action}
          <OfflineRegion region={region} />
          <button
            className="icon-button bordered"
            aria-label="Ampliar"
            onClick={() => zoomAt(1, size.width / 2, size.height / 2)}
          >
            <Plus size={15} />
          </button>
          <button
            className="icon-button bordered"
            aria-label="Reduzir"
            onClick={() => zoomAt(-1, size.width / 2, size.height / 2)}
          >
            <Minus size={15} />
          </button>
          <button
            className="icon-button bordered"
            aria-label="Enquadrar pontos"
            title="Enquadrar pontos"
            onClick={fit}
          >
            <Maximize2 size={15} />
          </button>
        </div>
      </div>
      <p className="map-note">
        <Info size={13} />
        Datum não confirmado; posição exploratória. Os tiles do OpenStreetMap
        revelam a região visível; os dados FC2 não são enviados.
        {tileError &&
          " Sem conexão com o provedor: os pontos continuam disponíveis."}
      </p>
      {unsupported > 0 && (
        <p className="map-note">
          {unsupported} coordenadas não representáveis em Web Mercator; consulte
          os valores na tabela.
        </p>
      )}
      {/* The canvas is always mounted so its size is always observed. */}
      <div
        ref={box}
        className="map-canvas"
        data-testid="map-canvas"
        onMouseDown={(e) => e.preventDefault()}
        onDragStart={(e) => e.preventDefault()}
        onPointerDown={(e) => {
          drag.current = { x: e.clientX, y: e.clientY, moved: 0 };
          e.currentTarget.setPointerCapture(e.pointerId);
        }}
        onPointerMove={(e) => {
          const d = drag.current;
          if (d) {
            const dx = e.clientX - d.x;
            const dy = e.clientY - d.y;
            d.moved += Math.abs(dx) + Math.abs(dy);
            d.x = e.clientX;
            d.y = e.clientY;
            setHover(null);
            setView((old) => {
              const w = TILE * 2 ** old.zoom;
              const nx = mercatorX(old.lon) * w - dx;
              const ny = mercatorY(old.lat) * w - dy;
              return {
                zoom: old.zoom,
                lon: wrap(unmercatorX(nx / w)),
                lat: unmercatorY(Math.min(1, Math.max(0, ny / w))),
              };
            });
            return;
          }
          const { x, y } = local(e);
          const hits = nearest(x, y);
          setHover((old) =>
            hits.length
              ? {
                  x,
                  y,
                  hits,
                  index:
                    old && old.hits[old.index]?.key === hits[old.index]?.key
                      ? old.index
                      : Math.max(
                          0,
                          hits.findIndex((h) => h.key === selectedKey),
                        ),
                }
              : null,
          );
        }}
        onPointerUp={(e) => {
          const d = drag.current;
          drag.current = null;
          if (d && d.moved < 4) {
            const { x, y } = local(e);
            const hits = nearest(x, y);
            if (hits.length) {
              const current = hits.findIndex((h) => h.key === selectedKey);
              const next = current >= 0 ? (current + 1) % hits.length : 0;
              onSelect(hits[next]);
              setHover({ x, y, hits, index: next });
            }
          }
        }}
        onPointerLeave={() => setHover(null)}
      >
        {tiles.map((t) => (
          <Tile
            key={t.key}
            id={t.src}
            style={{ left: t.x, top: t.y, width: t.s + 0.5, height: t.s + 0.5 }}
            onError={tileFailed}
          />
        ))}
        <svg
          className="map-markers"
          width={size.width}
          height={size.height}
          role="img"
          aria-label="Dispersão geográfica de longitude e latitude"
        >
          {ordered.map((p) => {
            const s = project(p.lon, p.lat);
            if (
              s.x < -8 ||
              s.y < -8 ||
              s.x > size.width + 8 ||
              s.y > size.height + 8
            )
              return null;
            const related = p.work === relatedWork;
            return (
              <circle
                key={p.key}
                className={`map-marker ${related ? "related" : ""}`}
                cx={s.x}
                cy={s.y}
                r={related ? 5 : 3.5}
              />
            );
          })}
          {selected &&
            (() => {
              const s = project(selected.lon, selected.lat);
              return (
                <circle
                  className="map-marker selected"
                  cx={s.x}
                  cy={s.y}
                  r={7}
                />
              );
            })()}
        </svg>
        {!points.length && (
          <div className="map-empty">
            <MapPin size={26} />
            <strong>Sem coordenadas disponíveis</strong>
            <span>
              Este conjunto não contém coordenadas numéricas disponíveis e
              válidas. Nenhum ponto foi criado.
            </span>
          </div>
        )}
        {shown && hover && (
          <PointCard
            point={shown}
            count={hover.hits.length}
            style={{
              left: Math.min(hover.x + 14, Math.max(size.width - 300, 0)),
              top: Math.min(hover.y + 14, Math.max(size.height - 270, 0)),
            }}
          />
        )}
      </div>
      <div className="map-footer">
        <span>© OpenStreetMap contributors (openstreetmap.org/copyright)</span>
        {selected && (
          <span>
            Registro {selected.id} · grupo {selected.group} · lat.{" "}
            {selected.lat} · lon. {selected.lon}
          </span>
        )}
      </div>
    </div>
  );
}

function PointCard({
  point,
  count,
  style,
}: {
  point: MapPoint;
  count: number;
  style: React.CSSProperties;
}) {
  const rows: [string, string][] = [
    ["ID do trabalho", String(point.workNumber)],
    ["Grupo", String(point.group)],
    ["Ponto", String(point.point)],
    ["Data", point.date ?? "—"],
    ["Hora", point.time ?? "—"],
    ["Índice a", point.a],
    ["Índice b", point.b],
    ["Índice total", point.total],
    ["Latitude", String(point.lat)],
    ["Longitude", String(point.lon)],
  ];
  return (
    <div className="map-card" style={style} role="tooltip">
      <strong>Registro {point.id}</strong>
      <small>{point.workName}</small>
      <dl>
        {rows.map(([k, v]) => (
          <div key={k}>
            <dt>{k}</dt>
            <dd>{v}</dd>
          </div>
        ))}
      </dl>
      <strong>Avisos ({point.warnings.length})</strong>
      {point.warnings.length === 0 ? (
        <small>Nenhum aviso neste registro.</small>
      ) : (
        point.warnings.map((w) => (
          <small key={w} className="warning-text" title={w}>
            {warningMessage(w)}
          </small>
        ))
      )}
      {count > 1 && (
        <small>{count} registros próximos; clique para alternar.</small>
      )}
      <small>
        Datum não confirmado; localização cartográfica exploratória.
      </small>
    </div>
  );
}

function Tile({
  id,
  style,
  onError,
}: {
  id: string;
  style: React.CSSProperties;
  onError: () => void;
}) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    let current = true;
    const [z, x, y] = id.split("/").map(Number);
    loadTile(z, x, y).then(
      (url) => current && setSrc(url),
      () => current && onError(),
    );
    return () => {
      current = false;
    };
  }, [id, onError]);
  return src ? (
    <img
      className="map-tile"
      src={src}
      alt=""
      draggable={false}
      style={style}
    />
  ) : null;
}
