import { useEffect, useRef, useState } from "react";
import { CloudDownload, LoaderCircle, X } from "lucide-react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { renderable, type MapPoint } from "./geo";

interface Plan {
  tiles: [number, number, number][];
  min_zoom: number;
  max_zoom: number;
  cached: number;
}
type Phase = "planning" | "ready" | "running" | "done" | "error";
const AVERAGE_TILE_KB = 18;

/** Pre-loads the OpenStreetMap tiles around the points of the open file into the disk cache. */
export function OfflineRegion({ region }: { region: MapPoint[] }) {
  const [open, setOpen] = useState(false);
  const [phase, setPhase] = useState<Phase>("planning");
  const [plan, setPlan] = useState<Plan | null>(null);
  const [message, setMessage] = useState("");
  const [progress, setProgress] = useState({
    done: 0,
    downloaded: 0,
    failed: 0,
  });
  const cancel = useRef(false);
  const box = useRef<HTMLDivElement>(null);
  const points = region.filter(renderable);

  useEffect(() => {
    if (!open) return;
    const away = (e: PointerEvent) => {
      if (!box.current?.contains(e.target as Node) && phase !== "running")
        setOpen(false);
    };
    document.addEventListener("pointerdown", away);
    return () => document.removeEventListener("pointerdown", away);
  }, [open, phase]);
  useEffect(() => () => void (cancel.current = true), []);

  async function show() {
    if (open) {
      setOpen(false);
      return;
    }
    setOpen(true);
    if (phase === "running") return;
    setPhase("planning");
    setMessage("");
    try {
      if (!isTauri()) throw "Disponível no aplicativo desktop.";
      let south = 90,
        north = -90,
        west = 180,
        east = -180;
      for (const p of points) {
        south = Math.min(south, p.lat);
        north = Math.max(north, p.lat);
        west = Math.min(west, p.lon);
        east = Math.max(east, p.lon);
      }
      const next = await invoke<Plan>("offline_plan", {
        bounds: { south, west, north, east },
      });
      setPlan(next);
      setPhase("ready");
    } catch (e) {
      setMessage(String(e));
      setPhase("error");
    }
  }
  async function download() {
    if (!plan) return;
    cancel.current = false;
    setPhase("running");
    setProgress({ done: 0, downloaded: 0, failed: 0 });
    let done = 0,
      downloaded = 0,
      failed = 0;
    for (const [z, x, y] of plan.tiles) {
      if (cancel.current) break;
      try {
        if (await invoke<boolean>("prefetch_tile", { z, x, y })) downloaded++;
      } catch {
        failed++;
      }
      done++;
      setProgress({ done, downloaded, failed });
    }
    if (cancel.current) {
      setMessage("Download interrompido; os tiles já baixados foram mantidos.");
      setPhase("ready");
    } else {
      setMessage(
        failed
          ? `Concluído com ${failed} tiles indisponíveis. Verifique a conexão e tente novamente.`
          : "Região disponível offline.",
      );
      setPhase(failed ? "ready" : "done");
    }
  }
  const missing = plan ? plan.tiles.length - plan.cached : 0;
  return (
    <div className="offline" ref={box}>
      <button
        className="button"
        onClick={() => void show()}
        disabled={!points.length}
        aria-expanded={open}
        title="Baixar a região deste arquivo para uso offline"
      >
        <CloudDownload size={15} />
        Baixar região offline
      </button>
      {open && (
        <div
          className="offline-popover"
          role="dialog"
          aria-label="Região offline"
        >
          <button
            className="icon-button close"
            aria-label="Fechar"
            disabled={phase === "running"}
            onClick={() => setOpen(false)}
          >
            <X size={14} />
          </button>
          <strong>Uso offline da região deste arquivo</strong>
          {phase === "planning" && (
            <p>
              <LoaderCircle size={14} className="spin" /> Calculando a área…
            </p>
          )}
          {plan && phase !== "planning" && phase !== "error" && (
            <>
              <p>
                Mapas do OpenStreetMap em zoom {plan.min_zoom} a {plan.max_zoom}{" "}
                ao redor dos {points.length.toLocaleString("pt-BR")} pontos:{" "}
                <b>{plan.tiles.length.toLocaleString("pt-BR")} tiles</b>
                {plan.cached > 0 && (
                  <>, {plan.cached.toLocaleString("pt-BR")} já no cache</>
                )}
                {missing > 0 && (
                  <>
                    {" "}
                    (≈{" "}
                    {Math.max(
                      1,
                      Math.round((missing * AVERAGE_TILE_KB) / 1024),
                    )}{" "}
                    MB a baixar)
                  </>
                )}
                .
              </p>
              {plan.max_zoom < 18 && (
                <p className="warning-text">
                  A área é grande: o zoom máximo foi limitado a {plan.max_zoom}{" "}
                  para manter o download pequeno.
                </p>
              )}
              <p className="muted">
                Os tiles são baixados um de cada vez, em ritmo reduzido, e só
                desta área, conforme a política de uso do OpenStreetMap. Os
                dados FC2 não são enviados.
              </p>
            </>
          )}
          {phase === "running" && plan && (
            <>
              <progress
                aria-label="Progresso do download"
                value={progress.done}
                max={plan.tiles.length}
              />
              <p>
                {progress.done.toLocaleString("pt-BR")} de{" "}
                {plan.tiles.length.toLocaleString("pt-BR")} tiles
                {progress.failed > 0 && ` · ${progress.failed} indisponíveis`}
              </p>
            </>
          )}
          {message && (
            <p
              role="status"
              className={phase === "error" ? "notice error" : ""}
            >
              {message}
            </p>
          )}
          <div className="offline-actions">
            {phase === "running" ? (
              <button
                className="button"
                onClick={() => (cancel.current = true)}
              >
                Interromper
              </button>
            ) : (
              <button
                className="button primary"
                disabled={phase !== "ready" || missing === 0}
                onClick={() => void download()}
              >
                <CloudDownload size={15} />
                {phase === "done"
                  ? "Concluído"
                  : phase === "ready" && missing === 0
                    ? "Já disponível offline"
                    : "Baixar agora"}
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
