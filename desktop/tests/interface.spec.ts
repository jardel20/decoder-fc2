import { test, expect, type Page } from "@playwright/test";

// Synthetic presentation data. Native bridge and binary exports are tested separately in Rust.
const source = {
  name: "synthetic.FC2",
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
    page.getByRole("heading", { name: "Quais trabalhos vamos explorar?" }),
  ).toBeVisible();
}

test("welcome is responsive, supports themes and keyboard-accessible help", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Continuar", exact: true }),
  ).toBeDisabled();
  await page.screenshot({
    path: "test-results/welcome-light.png",
    fullPage: true,
    animations: "disabled",
  });
  await page.getByRole("button", { name: "Ativar tema escuro" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.screenshot({
    path: "test-results/welcome-dark.png",
    fullPage: true,
    animations: "disabled",
  });
  await page.getByRole("button", { name: "Ajuda", exact: true }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.setViewportSize({ width: 800, height: 620 });
  await expect(
    page.getByRole("button", { name: "Selecionar arquivo FC2" }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBeTruthy();
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBeTruthy();
  await page.screenshot({
    path: "test-results/welcome-mobile.png",
    fullPage: true,
    animations: "disabled",
  });
});

test("select, inspect, filter, paginate and export using the source identity", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await openFixture(page);
  await page.screenshot({
    path: "test-results/works.png",
    fullPage: true,
    animations: "disabled",
  });
  await page
    .getByRole("textbox", { name: "Buscar trabalhos" })
    .fill("controle");
  await expect(
    page.getByRole("checkbox", { name: "Selecionar Controle sintético" }),
  ).toBeVisible();
  await expect(
    page.getByRole("checkbox", { name: "Selecionar Ensaio sintético" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Limpar busca" }).click();
  await page.getByRole("button", { name: "Limpar", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Continuar", exact: true }),
  ).toBeDisabled();
  await page
    .getByRole("checkbox", { name: "Selecionar Ensaio sintético" })
    .check();
  await page.getByRole("button", { name: "Continuar", exact: true }).click();
  await expect(
    page.getByRole("tab", { name: "Medições", exact: true }),
  ).toBeVisible();
  await expect(page.getByText("1 de 2", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Próxima página" }).click();
  await expect(page.getByText("2 de 2", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Página anterior" }).click();
  await page
    .getByRole("button", {
      name: "Inspecionar medição 1 do grupo 1",
      exact: true,
    })
    .click();
  await expect(page.getByRole("dialog")).toContainText(
    "calendar_all_zero_raw_values_preserved",
  );
  await expect(page.getByRole("dialog")).toContainText("0x430");
  await page.keyboard.press("Escape");
  await page.getByLabel("Filtrar grupo").selectOption("2");
  await expect(page.locator(".table-bottom")).toContainText("4 medições");
  await page.getByRole("tab", { name: "Gráfico", exact: true }).click();
  await expect(
    page.getByRole("img", { name: "Gráfico de 4 medições de clorofila" }),
  ).toBeVisible();
  await page.getByRole("tab", { name: "Coordenadas", exact: true }).click();
  await expect(
    page.getByText("Datum não confirmado.", { exact: false }),
  ).toBeVisible();
  await page.getByRole("tab", { name: "Origem e avisos" }).click();
  await expect(page.getByText(source.sha256, { exact: true })).toBeVisible();
  await page.getByRole("tab", { name: "Medições", exact: true }).click();
  await page.screenshot({
    path: "test-results/explore.png",
    fullPage: true,
    animations: "disabled",
  });
  await page.getByRole("button", { name: "Preparar exportação" }).click();
  await page.screenshot({
    path: "test-results/export.png",
    fullPage: true,
    animations: "disabled",
  });
  await page.getByRole("radio", { name: /Auditoria JSON/ }).check();
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

test("JSON requires one selection, partial failures remain visible", async ({
  page,
}) => {
  await mockBridge(page, { partial: true });
  await page.goto("/");
  await page.getByRole("button", { name: "Selecionar arquivo FC2" }).click();
  await page.getByRole("button", { name: /04 Exportar/ }).click();
  await page.getByRole("radio", { name: /Auditoria JSON/ }).check();
  await expect(
    page.getByRole("button", { name: "Escolher destino e exportar" }),
  ).toBeDisabled();
  await page.getByRole("radio", { name: /Pacote completo/ }).check();
  await page
    .getByRole("button", { name: "Escolher destino e exportar" })
    .click();
  await expect(page.getByRole("status")).toContainText(
    "20: inconsistent groups",
  );
});

for (const option of ["cancel", "fail"] as const)
  test(`opening handles ${option} without losing the welcome screen`, async ({
    page,
  }) => {
    await mockBridge(page, { [option]: true });
    await page.goto("/");
    await page.getByRole("button", { name: "Selecionar arquivo FC2" }).click();
    await expect(
      page.getByRole("heading", { name: "Seus dados começam aqui." }),
    ).toBeVisible();
    if (option === "fail")
      await expect(page.getByRole("alert")).toContainText(
        "assinatura não reconhecida",
      );
    await expect(
      page.getByRole("button", { name: "Selecionar arquivo FC2" }),
    ).toBeEnabled();
  });

test("work detail errors offer recovery and missing coordinates stay empty", async ({
  page,
}) => {
  await mockBridge(page, { workFail: true });
  await page.goto("/");
  await page.getByRole("button", { name: "Selecionar arquivo FC2" }).click();
  await page.getByRole("button", { name: "Continuar", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Falha temporária");
  await page.getByRole("button", { name: "Tentar novamente" }).click();
  await expect(page.getByText("1 de 2", { exact: true })).toBeVisible();
  await page.getByLabel("Trabalho em exploração").selectOption("1");
  await page.getByRole("tab", { name: "Coordenadas", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Sem coordenadas disponíveis" }),
  ).toBeVisible();
});
