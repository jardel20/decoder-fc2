// Tiles come from the Rust side (User-Agent, disk cache). At most two requests run at
// once, in line with the OpenStreetMap tile usage policy.
import { invoke, isTauri } from "@tauri-apps/api/core";

const cache = new Map<string, Promise<string>>();
const waiting: (() => void)[] = [];
let running = 0;
const LIMIT = 2;
const MAX_CACHED = 600;

function pump() {
  while (running < LIMIT && waiting.length) waiting.shift()!();
}
export function loadTile(z: number, x: number, y: number): Promise<string> {
  const key = `${z}/${x}/${y}`;
  const hit = cache.get(key);
  if (hit) return hit;
  const promise = new Promise<string>((resolve, reject) => {
    waiting.push(() => {
      running++;
      (isTauri()
        ? invoke<string>("get_tile", { z, x, y })
        : Promise.reject("Mapas online exigem o aplicativo desktop.")
      )
        .then(resolve, reject)
        .finally(() => {
          running--;
          pump();
        });
    });
    pump();
  });
  // Failed tiles are retried on the next request.
  promise.catch(() => cache.delete(key));
  cache.set(key, promise);
  if (cache.size > MAX_CACHED) cache.delete(cache.keys().next().value!);
  return promise;
}
