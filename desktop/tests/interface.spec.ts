import { test, expect, type Page } from "@playwright/test";

// Synthetic presentation data. Native bridge and binary exports are tested separately in Rust.
const source = {
  name: "synthetic.FC2",
  path: "/tmp/synthetic.FC2",
  crc_verified: 12,
  sha256: "a".repeat(64),
  size: 2097152,
  records: 81,
  groups: 3,
  coordinates: 79,
  limitations: ["Synthetic fixture. Datum não confirmado."],
  works: [
    {
      key: 0,
      number: 10,
      name: "Ensaio sintético",
      records: 80,
      groups: 2,
      points: 40,
      coordinates: 79,
      warnings: 1,
      errors: [],
      directory: "/trabalho10",
      notes: [],
      first_date: "2024-03-12",
      last_date: "2024-03-13",
    },
    {
      key: 1,
      number: 20,
      name: "Controle sintético",
      records: 1,
      groups: 1,
      points: 1,
      coordinates: 0,
      warnings: 0,
      errors: [],
      directory: "/trabalho20",
      notes: [],
      first_date: null,
      last_date: null,
    },
  ],
};
const record = (i: number) => ({
  id: i,
  point: Math.ceil(i / 2),
  date: i === 1 ? null : "2024-03-12",
  time: i === 1 ? null : "10:30:00",
  latitude: i === 1 ? null : -12.25 + i * 0.001,
  longitude: i === 1 ? null : 34.5 + i * 0.001,
  chlorophyll_a_tenths: 123 + (i % 2),
  chlorophyll_b_tenths: 45,
  chlorophyll_total_tenths: 169 + (i % 2),
  chlorophyll_a_index: "12.3",
  chlorophyll_b_index: "4.5",
  chlorophyll_total_index: "16.9",
  raw_hex: "ab".repeat(48),
  raw_sha256: "b".repeat(64),
  segments: [
    { file_offset: 1024 + i * 48, memory_address: i * 48, length: 48 },
  ],
  warnings: i === 1 ? ["calendar_all_zero_raw_values_preserved"] : [],
});
async function mockBridge(
  page: Page,
  options: {
    cancel?: boolean;
    fail?: boolean;
    partial?: boolean;
    workFail?: boolean;
  } = {},
) {
  await page.addInitScript(
    ({ source, records, options }) => {
      let nextId = 1;
      let failedWork = false;
      const callbacks = new Map<number, (v: unknown) => void>();
      const internals = {
        transformCallback: (callback: (v: unknown) => void) => {
          const id = nextId++;
          callbacks.set(id, callback);
          return id;
        },
        unregisterCallback: (id: number) => callbacks.delete(id),
        invoke: async (cmd: string, args: Record<string, unknown>) => {
          if (cmd === "initial_path") return null;
          if (cmd === "open_tui") return null;
          if (cmd === "offline_plan") {
            (window as any).__plan = args;
            return {
              tiles: Array.from({ length: 12 }, (_, i) => [12, 100 + i, 200]),
              min_zoom: 10,
              max_zoom: 16,
              cached: 2,
            };
          }
          if (cmd === "prefetch_tile") {
            (window as any).__prefetched =
              ((window as any).__prefetched || 0) + 1;
            return true;
          }
          if (cmd === "get_tile") {
            (window as any).__tiles = ((window as any).__tiles || 0) + 1;
            return "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";
          }
          if (cmd === "get_locations")
            return (records as any[])
              .map((r, record) => ({ r, record }))
              .filter(({ r }) => r.latitude != null)
              .map(({ r, record }) => ({
                work: 0,
                work_number: 10,
                work_name: "Ensaio sintético",
                group_index: record < 76 ? 0 : 1,
                group: record < 76 ? 1 : 2,
                record: record < 76 ? record : record - 76,
                id: r.id,
                point: r.point,
                date: r.date,
                time: r.time,
                a: r.chlorophyll_a_index,
                b: r.chlorophyll_b_index,
                total: r.chlorophyll_total_index,
                latitude: r.latitude,
                longitude: r.longitude,
                warnings: r.warnings,
              }));
          if (cmd === "open_source") {
            if (options.fail)
              throw "Arquivo FC2 inválido: assinatura não reconhecida.";
            return options.cancel ? null : source;
          }
          if (cmd === "get_work" && options.workFail && !failedWork) {
            failedWork = true;
            throw "Falha temporária ao carregar o trabalho.";
          }
          if (cmd === "get_work")
            return {
              number: args.key === 0 ? 10 : 20,
              display_name:
                args.key === 0 ? "Ensaio sintético" : "Controle sintético",
              directory: "/trabalho10",
              name_source: "configuration",
              configuration_file: "/trabalho10/config_measures",
              warnings: [],
              errors: [],
              groups:
                args.key === 0
                  ? [
                      {
                        group: 1,
                        source_file: "/trabalho10/1",
                        records: records.slice(0, 76),
                      },
                      {
                        group: 2,
                        source_file: "/trabalho10/2",
                        records: records.slice(76),
                      },
                    ]
                  : [
                      {
                        group: 1,
                        source_file: "/trabalho20/1",
                        records: [records[0]],
                      },
                    ],
            };
          if (cmd === "export_selection") {
            (window as any).__lastExport = args;
            return {
              path: "/tmp/synthetic-export",
              workbooks: options.partial ? 1 : (args.keys as number[]).length,
              records: 81,
              failures: options.partial ? 1 : 0,
              errors: options.partial ? ["20: inconsistent groups"] : [],
            };
          }
          if (cmd === "plugin:event|listen") return 1;
          if (cmd === "plugin:event|unlisten") return;
          throw `Unexpected command: ${cmd}`;
        },
        metadata: {
          currentWindow: { label: "main" },
          currentWebview: { label: "main" },
        },
      };
      Object.defineProperty(window, "isTauri", { value: true });
      Object.defineProperty(window, "__TAURI_INTERNALS__", {
        value: internals,
      });
      Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", {
        value: { unregisterListener: () => {} },
      });
    },
    {
      source,
      records: Array.from({ length: 80 }, (_, i) => record(i + 1)),
      options,
    },
  );
}
async function openFixture(page: Page) {
  await mockBridge(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Selecionar arquivo FC2" }).click();
  await expect(
    page.getByRole("heading", { name: "Visão geográfica" }),
  ).toBeVisible();
}
/** Themed select: open the combobox, then click the option. */
async function pick(page: Page, label: string, option: string | RegExp) {
  await page.getByRole("combobox", { name: label }).click();
  await page.getByRole("option", { name: option }).click();
}
/** Select the work in the explorer, then open it. */
async function openWork(page: Page, name = "Ensaio sintético") {
  await page.getByRole("button", { name: `Trabalho ${name}` }).click();
  await page.getByRole("button", { name: "Abrir trabalho" }).click();
  await expect(page.getByRole("heading", { name })).toBeVisible();
}

test("empty view, themes and keyboard help", async ({ page }) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Selecionar arquivo FC2" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Ativar tema escuro" }).click();
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.getByRole("button", { name: "Ajuda", exact: true }).click();
  await expect(page.getByRole("dialog")).toContainText("OpenStreetMap");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
});

test("explorer: overview map with all points, selection highlights the work, double click opens", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await openFixture(page);
  await expect(page.locator(".map-panel .chart-heading p")).toContainText(
    "79 coordenadas",
  );
  await expect(page.locator(".map-tile").first()).toBeAttached();
  expect(await page.locator(".map-marker").count()).toBeGreaterThan(10);
  expect(await page.locator(".map-marker.related").count()).toBe(0);
  await expect(page.getByText("Selecione um trabalho na lista")).toBeVisible();
  await page.screenshot({
    path: "test-results/explorer-initial.png",
    animations: "disabled",
  });
  // Selecting a work highlights its points and fills the detail panel.
  await page.getByRole("button", { name: "Trabalho Ensaio sintético" }).click();
  await expect(page.locator(".explorer-details")).toContainText(
    "ID 10 • 80 registros • 2 grupos",
  );
  await expect
    .poll(() => page.locator(".map-marker.related").count())
    .toBeGreaterThan(10);
  // Clicking a marker selects the record and its work.
  const marker = page.locator(".map-marker.related").first();
  const box = (await marker.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await expect(page.getByRole("tooltip")).toContainText("Registro");
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  await expect(page.locator(".map-marker.selected")).toHaveCount(1);
  await expect(page.locator(".explorer-details")).toContainText(
    "Inspecionar registro",
  );
  await page.screenshot({
    path: "test-results/explorer-selected.png",
    animations: "disabled",
  });
  await page.getByRole("button", { name: "Inspecionar registro" }).click();
  await expect(page.getByRole("dialog")).toContainText("Grupo");
  await page.keyboard.press("Escape");
  // Double click opens the work; Voltar returns to the explorer.
  await page.locator(".work-item").first().dblclick();
  await expect(page.getByRole("tab", { name: "Medições" })).toBeVisible();
  await page.getByRole("button", { name: "Voltar" }).click();
  await expect(
    page.getByRole("heading", { name: "Visão geográfica" }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});

test("work list: marks, marked-first, sort, select all, reload, CRC and export of marked", async ({
  page,
}) => {
  await openFixture(page);
  const names = () => page.locator(".work-name").allTextContents();
  expect(await names()).toEqual([
    "10 · Ensaio sintético",
    "20 · Controle sintético",
  ]);
  await pick(page, "Ordenar trabalhos", "Ordenar: nome");
  expect(await names()).toEqual([
    "20 · Controle sintético",
    "10 · Ensaio sintético",
  ]);
  await page.getByRole("checkbox", { name: "Decrescente" }).check();
  expect(await names()).toEqual([
    "10 · Ensaio sintético",
    "20 · Controle sintético",
  ]);
  await expect(
    page.getByRole("button", { name: "Exportar marcados" }),
  ).toBeDisabled();
  await page
    .getByRole("checkbox", { name: "Marcar Controle sintético" })
    .check();
  await expect(page.getByText("Selecionados (1)")).toBeVisible();
  expect((await names())[0]).toBe("20 · Controle sintético");
  await expect(page.getByText("2 visíveis • 1 marcados")).toBeVisible();
  await page
    .getByRole("checkbox", { name: "Marcar todos os trabalhos" })
    .check();
  await expect(page.getByText("2 visíveis • 2 marcados")).toBeVisible();
  await page.getByRole("button", { name: "Limpar marcações" }).click();
  await page
    .getByRole("textbox", { name: "Buscar trabalhos" })
    .fill("controle");
  await expect(
    page.getByRole("checkbox", { name: "Marcar Ensaio sintético" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Limpar busca" }).click();
  await expect(page.locator(".statusbar")).toContainText(
    "CRC conferido · 12 setores",
  );
  // Export marked: JSON needs exactly one; partial failures are reported.
  await page
    .getByRole("checkbox", { name: "Marcar todos os trabalhos" })
    .check();
  await pick(page, "Formato de exportação", /JSON/);
  await page.getByRole("button", { name: "Exportar marcados" }).click();
  await expect(page.getByRole("alert")).toContainText("único trabalho");
  await page.getByRole("button", { name: "Fechar aviso" }).click();
  await pick(page, "Formato de exportação", /Completo/);
  await page.getByRole("button", { name: "Exportar marcados" }).click();
  await expect(page.getByRole("status")).toContainText("Exportação concluída");
  expect(await page.evaluate(() => (window as any).__lastExport)).toEqual({
    source: source.sha256,
    keys: [0, 1],
    mode: "complete",
  });
  await page.getByRole("button", { name: "Recarregar o FC2 do disco" }).click();
  await expect(page.getByText("Selecione um trabalho na lista")).toBeVisible();
});

test("work view: filter, sort, inspect, charts, map, metadata and per-work export", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await openFixture(page);
  await openWork(page);
  await expect(page.getByText("1 de 2", { exact: true })).toBeVisible();
  await page.screenshot({
    path: "test-results/workspace-light.png",
    animations: "disabled",
  });
  await page.getByRole("button", { name: "Próxima página" }).click();
  await expect(page.getByText("2 de 2", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Página anterior" }).click();
  await page.locator("tbody tr").first().click();
  await expect(page.getByRole("dialog")).toContainText(
    "Data e hora não estão preenchidas",
  );
  await expect(page.getByRole("dialog")).toContainText(
    "calendar_all_zero_raw_values_preserved",
  );
  await expect(page.getByRole("dialog")).toContainText("0x430");
  await page.keyboard.press("Escape");
  await page.getByRole("checkbox", { name: "Somente avisos" }).check();
  await expect(page.locator("tbody tr")).toHaveCount(1);
  await page.getByRole("checkbox", { name: "Somente avisos" }).uncheck();
  await page.getByRole("textbox", { name: "Buscar medições" }).fill("10:30");
  await expect(page.locator(".record-count")).toHaveText("79 / 80 registros");
  await page.getByRole("textbox", { name: "Buscar medições" }).fill("");
  await page.getByRole("button", { name: "ID", exact: true }).click();
  await page.getByRole("button", { name: "ID", exact: true }).click();
  await expect(
    page.locator("tbody tr").first().locator("td").first(),
  ).toHaveText("80");
  await pick(page, "Filtrar grupo", "Grupo 2");
  await expect(page.locator("tbody tr")).toHaveCount(4);
  await page.getByRole("tab", { name: "Gráficos", exact: true }).click();
  await expect(
    page.getByRole("img", { name: "Gráfico de 2 pontos de clorofila" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Todos os pontos" }).click();
  await expect(
    page.getByRole("img", { name: "Gráfico de 4 leituras de clorofila" }),
  ).toBeVisible();
  await page.locator(".bar").first().hover();
  await expect(page.getByRole("tooltip")).toContainText("ID 77");
  await page.getByRole("tab", { name: "Mapa", exact: true }).click();
  await expect(page.locator(".map-panel .chart-heading p")).toContainText(
    "4 coordenadas",
  );
  await page.getByRole("tab", { name: "Metadados", exact: true }).click();
  await expect(page.getByText(source.sha256, { exact: true })).toBeVisible();
  await page.getByRole("tab", { name: "Exportação", exact: true }).click();
  await page.getByRole("radio", { name: /JSON/ }).click();
  await page
    .getByRole("button", { name: "Escolher destino e exportar" })
    .click();
  await expect(page.getByRole("status")).toContainText("Auditoria JSON salva.");
  expect(await page.evaluate(() => (window as any).__lastExport)).toEqual({
    source: source.sha256,
    keys: [0],
    mode: "json",
  });
  expect(errors).toEqual([]);
});

test("charts: mean ± SE, variables, zoom and shared filters", async ({
  page,
}) => {
  await openFixture(page);
  await openWork(page);
  await page.getByRole("tab", { name: "Gráficos", exact: true }).click();
  await expect(
    page.getByRole("img", { name: "Gráfico de 40 pontos de clorofila" }),
  ).toBeVisible();
  expect(await page.locator(".bar").count()).toBe(40);
  expect(await page.locator(".error-bar").count()).toBe(40);
  await page.locator(".bar").first().hover();
  await expect(page.getByRole("tooltip")).toContainText("Erro padrão");
  await expect(page.getByRole("tooltip")).toContainText("n: 2 leituras");
  await pick(page, "Série do gráfico", "Todas");
  expect(await page.locator(".bar").count()).toBe(120);
  const before = await page
    .locator(".bar-chart")
    .evaluate((e) => e.clientWidth);
  await page.getByRole("button", { name: "Aproximar gráfico" }).click();
  expect(
    await page.locator(".bar-chart").evaluate((e) => e.clientWidth),
  ).toBeGreaterThan(before);
  await page.getByRole("button", { name: "Ajustar à largura" }).click();
  await page.screenshot({
    path: "test-results/charts.png",
    animations: "disabled",
  });
});

test("work map: tiles via Rust, selection without drag-select and show-on-map from the inspector", async ({
  page,
}) => {
  await mockBridge(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Selecionar arquivo FC2" }).click();
  await openWork(page);
  await page.getByRole("tab", { name: "Mapa", exact: true }).click();
  await expect(page.locator(".map-panel .chart-heading p")).toContainText(
    "79 coordenadas",
  );
  const tiles = () => page.evaluate(() => (window as any).__tiles || 0);
  await expect.poll(tiles).toBeGreaterThan(0);
  const zoomText = await page
    .getByText(/zoom \d/)
    .first()
    .textContent();
  await page.getByRole("button", { name: "Ampliar" }).click();
  await expect(page.getByText(/zoom \d/).first()).not.toHaveText(zoomText!);
  await page.getByRole("button", { name: "Enquadrar pontos" }).click();
  const marker = page.locator(".map-marker").first();
  const box = (await marker.boundingBox())!;
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  await expect(page.locator(".map-marker.selected")).toHaveCount(1);
  await expect(page.locator(".map-footer")).toContainText("lat.");
  await page.screenshot({
    path: "test-results/map.png",
    animations: "disabled",
  });
  await page.getByRole("tab", { name: "Medições", exact: true }).click();
  await page.locator("tbody tr").nth(1).click();
  await page.getByRole("button", { name: "Mostrar no mapa" }).click();
  await expect(page.getByText(/zoom 17/).first()).toBeVisible();
});

test("work without coordinates and independent selection; partial export failure", async ({
  page,
}) => {
  await mockBridge(page, { partial: true });
  await page.goto("/");
  await page.getByRole("button", { name: "Selecionar arquivo FC2" }).click();
  await page
    .getByRole("checkbox", { name: "Marcar todos os trabalhos" })
    .check();
  await openWork(page, "Controle sintético");
  await page.getByRole("tab", { name: "Mapa", exact: true }).click();
  await expect(page.getByText("Sem coordenadas disponíveis")).toBeVisible();
  await page.getByRole("button", { name: "Voltar" }).click();
  await page.getByRole("button", { name: "Exportar marcados" }).click();
  await expect(page.getByRole("status")).toContainText(
    "20: inconsistent groups",
  );
});

for (const option of ["cancel", "fail"] as const)
  test(`file open handles ${option}`, async ({ page }) => {
    await mockBridge(page, { [option]: true });
    await page.goto("/");
    await page.getByRole("button", { name: "Selecionar arquivo FC2" }).click();
    await expect(
      page.getByRole("heading", { name: "Abrir arquivo FC2" }),
    ).toBeVisible();
    if (option === "fail")
      await expect(page.getByRole("alert")).toContainText(
        "assinatura não reconhecida",
      );
    await expect(
      page.getByRole("button", { name: "Selecionar arquivo FC2" }),
    ).toBeEnabled();
  });

test("retry work detail; layouts at minimum and wide window sizes", async ({
  page,
}) => {
  await mockBridge(page, { workFail: true });
  await page.goto("/");
  await page.getByRole("button", { name: "Selecionar arquivo FC2" }).click();
  await page.getByRole("button", { name: "Trabalho Ensaio sintético" }).click();
  await page.getByRole("button", { name: "Abrir trabalho" }).click();
  await expect(page.getByRole("alert")).toContainText("Falha temporária");
  await page.getByRole("button", { name: "Tentar novamente" }).click();
  await expect(page.getByText("1 de 2", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Ativar tema escuro" }).click();
  await page.screenshot({
    path: "test-results/workspace-dark.png",
    animations: "disabled",
  });
  await page.getByRole("button", { name: "Voltar" }).click();
  for (const [width, height] of [
    [800, 620],
    [1900, 1026],
  ]) {
    await page.setViewportSize({ width, height });
    expect(
      await page.evaluate(
        () =>
          document.documentElement.scrollWidth <= innerWidth &&
          document.documentElement.scrollHeight <= innerHeight,
      ),
    ).toBeTruthy();
    await expect(
      page.getByRole("button", { name: "Abrir trabalho" }),
    ).toBeVisible();
    await expect(page.getByTestId("map-canvas")).toBeVisible();
    await page.screenshot({
      path: `test-results/explorer-${width}.png`,
      animations: "disabled",
    });
  }
});

test("themed selects, column sort/filter popovers and drag does not select", async ({
  page,
}) => {
  await openFixture(page);
  // The list is rendered by the app, so it follows the app theme.
  await page.getByRole("combobox", { name: "Ordenar trabalhos" }).click();
  const listbox = page.getByRole("listbox");
  await expect(listbox).toBeVisible();
  const [bg, fg] = await listbox.evaluate((e) => {
    const c = getComputedStyle(e);
    return [c.backgroundColor, c.color];
  });
  expect(bg).toBe("rgb(255, 255, 255)");
  expect(fg).not.toBe(bg);
  await page.keyboard.press("Escape");
  await expect(listbox).toHaveCount(0);
  await page.getByRole("button", { name: "Ativar tema escuro" }).click();
  await page.getByRole("combobox", { name: "Ordenar trabalhos" }).click();
  expect(
    await page
      .getByRole("listbox")
      .evaluate((e) => getComputedStyle(e).backgroundColor),
  ).not.toBe("rgb(255, 255, 255)");
  await page.keyboard.press("Escape");
  // The top bar follows the theme.
  expect(
    await page
      .locator(".topbar")
      .evaluate((e) => getComputedStyle(e).backgroundColor),
  ).toBe(
    await page.evaluate(() =>
      getComputedStyle(document.documentElement)
        .getPropertyValue("--surface")
        .trim() === "#2f2f2f"
        ? "rgb(47, 47, 47)"
        : "",
    ),
  );
  // Dragging the map never selects a record.
  const canvas = page.getByTestId("map-canvas");
  const box = (await canvas.boundingBox())!;
  await page.mouse.move(box.x + 60, box.y + 60);
  await page.mouse.down();
  await page.mouse.move(box.x + 200, box.y + 150, { steps: 6 });
  await page.mouse.up();
  await expect(page.locator(".map-marker.selected")).toHaveCount(0);
  await openWork(page);
  // Column headers: sort icon plus a filter funnel; filters apply to rows.
  await expect(
    page.getByRole("button", { name: "Filtrar Total" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Filtrar ID" }).click();
  await page.getByLabel("Mínimo").fill("10");
  await page.getByLabel("Máximo").fill("19");
  await page.screenshot({
    path: "test-results/column-filter.png",
    animations: "disabled",
  });
  await expect(page.locator(".record-count")).toHaveText("10 / 80 registros");
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("button", { name: /Limpar 1 filtro/ }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Filtrar Avisos" }).click();
  await page.getByRole("button", { name: "Com avisos" }).click();
  await expect(page.locator(".record-count")).toHaveText("0 / 80 registros");
  await page.getByRole("button", { name: /Limpar 2 filtros/ }).click();
  await expect(page.locator(".record-count")).toHaveText("80 / 80 registros");
  // No separate inspect column; the row opens the record.
  await expect(page.locator("th")).toHaveCount(10);
  await page.locator("tbody tr").first().click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  // Export options are cards with title and description.
  await page.getByRole("tab", { name: "Exportação", exact: true }).click();
  await expect(page.getByRole("radio")).toHaveCount(3);
  await expect(page.getByRole("radio", { name: /Completo/ })).toContainText(
    "rastreabilidade",
  );
  await page.screenshot({
    path: "test-results/export-cards.png",
    animations: "disabled",
  });
  // Help shows the licences without a collapsible section.
  await page.getByRole("button", { name: "Ajuda", exact: true }).click();
  await expect(page.getByRole("dialog")).toContainText("Ícones Lucide");
  await page.screenshot({
    path: "test-results/help.png",
    animations: "disabled",
  });
  await expect(page.locator("details")).toHaveCount(0);
  // Text never goes below 12px.
  const tiny = await page.evaluate(
    () =>
      [...document.querySelectorAll("body *")].filter((e) => {
        const el = e as HTMLElement;
        return (
          el.childNodes.length &&
          [...el.childNodes].some(
            (n) => n.nodeType === 3 && n.textContent!.trim(),
          ) &&
          parseFloat(getComputedStyle(el).fontSize) < 12
        );
      }).length,
  );
  expect(tiny).toBe(0);
});

test("map click reveals the work in the list; chart fits width; tabs show no focus ring; app name", async ({
  page,
}) => {
  await openFixture(page);
  await expect(page.locator(".brand")).toContainText("Leaf Explorer");
  await expect(page).toHaveTitle("Leaf Explorer");
  // Clicking a point of a work selects it in the left list as well.
  await page
    .getByRole("textbox", { name: "Buscar trabalhos" })
    .fill("controle");
  const marker = page.locator(".map-marker").first();
  const box = (await marker.boundingBox())!;
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  await expect(page.locator(".work-item.active")).toContainText(
    "Ensaio sintético",
  );
  await expect(
    page.getByRole("textbox", { name: "Buscar trabalhos" }),
  ).toHaveValue("");
  await expect(page.locator(".work-item.active")).toBeInViewport();
  await openWork(page);
  // Tabs: clicking never leaves a focus outline clipped by the tab strip.
  await page.getByRole("tab", { name: "Gráficos", exact: true }).click();
  expect(
    await page
      .getByRole("tab", { name: "Gráficos", exact: true })
      .evaluate((e) => getComputedStyle(e).outlineStyle),
  ).toBe("none");
  // "Ajustar à largura" shows every bar without horizontal scrolling.
  const scroller = page.locator(".chart-scroll");
  await page.getByRole("button", { name: "Aproximar gráfico" }).click();
  await page.getByRole("button", { name: "Aproximar gráfico" }).click();
  await page.getByRole("button", { name: "Aproximar gráfico" }).click();
  expect(await scroller.evaluate((e) => e.scrollWidth > e.clientWidth)).toBe(
    true,
  );
  await page.getByRole("button", { name: "Ajustar à largura" }).click();
  expect(
    await scroller.evaluate((e) => e.scrollWidth <= e.clientWidth + 1),
  ).toBe(true);
  await page.getByRole("button", { name: "Todos os pontos" }).click();
  await page.getByRole("button", { name: "Ajustar à largura" }).click();
  expect(
    await scroller.evaluate((e) => e.scrollWidth <= e.clientWidth + 1),
  ).toBe(true);
});

test("offline region: plan from the points of the file, progress, cancel and completion", async ({
  page,
}) => {
  await openFixture(page);
  await page.getByRole("button", { name: "Baixar região offline" }).click();
  const dialog = page.getByRole("dialog", { name: "Região offline" });
  await expect(dialog).toContainText("12 tiles");
  await expect(dialog).toContainText("2 já no cache");
  await expect(dialog).toContainText("zoom 10 a 16");
  await expect(dialog).toContainText("limitado a 16");
  const plan = await page.evaluate(() => (window as any).__plan);
  expect(plan.bounds.south).toBeLessThan(plan.bounds.north);
  expect(plan.bounds.west).toBeLessThan(plan.bounds.east);
  await page.screenshot({
    path: "test-results/offline.png",
    animations: "disabled",
  });
  await dialog.getByRole("button", { name: "Baixar agora" }).click();
  await expect(dialog.getByRole("status")).toContainText(
    "Região disponível offline",
  );
  expect(await page.evaluate(() => (window as any).__prefetched)).toBe(12);
  await expect(
    dialog.getByRole("button", { name: "Concluído" }),
  ).toBeDisabled();
});
