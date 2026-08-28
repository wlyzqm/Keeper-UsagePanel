import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { createServer } from "vite";
import { chromium } from "playwright-core";
await mkdir(".cache/screenshots", { recursive: true });
const server = await createServer({
  server: { host: "127.0.0.1", port: 1420 },
});
await server.listen();
let browser;
try {
  browser = await chromium.launch({
    executablePath: process.env.CHROMIUM_PATH || "/usr/bin/chromium",
    headless: true,
    args: ["--no-sandbox", "--disable-dev-shm-usage", "--disable-gpu"],
  });
  const page = await browser.newPage({
    viewport: { width: 1024, height: 820 },
  });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("http://127.0.0.1:1420/?preview=1");
  await page.locator(".metric-value").first().waitFor();
  await page.screenshot({ path: ".cache/screenshots/overview-light.png" });
  assert.equal(await page.locator(".settings-button").innerText(), "设置");
  await page.locator('[data-console="usage"]').click();
  await page.locator('[data-console="cpa"]').click();
  assert.deepEqual(
    await page.evaluate(() =>
      window.__previewCalls
        .filter((c) => c.command === "open_console")
        .map((c) => c.args.target),
    ),
    ["usage", "cpa"],
  );

  const labelRect = await page.locator(".metric-label").first().boundingBox();
  await page.mouse.move(labelRect.x + 2, labelRect.y + 8);
  await page.mouse.down();
  await page.mouse.move(labelRect.x + labelRect.width - 2, labelRect.y + 8, {
    steps: 8,
  });
  await page.mouse.up();
  assert.equal(
    await page.evaluate(() => window.getSelection().toString()),
    "",
    "Static panel text cannot be selected",
  );
  assert.equal(await page.locator("#today-total").innerText(), "295M");
  assert.equal(await page.locator("#delta-input").innerText(), "12.5K");
  await page.locator('[data-pick=range][data-value="30d"]').click();
  await page.locator(".scope-tag").filter({ hasText: "近 30 天" }).waitFor();
  await page.locator('summary[aria-label="选择 Key owner"]').click();
  await page.locator("[data-pick=key][data-value=k1]").click();
  await page.locator(".scope-tag").filter({ hasText: "开发项目" }).waitFor();
  await page.waitForFunction(
    () => document.querySelector("#today-total").textContent === "1.23M",
  );
  assert.equal(await page.locator("[data-tab=accounts]").count(), 0);
  await page.evaluate(() =>
    window.__previewEmitSample({
      revision: 0,
      sampled_at: "2030-01-01T00:00:00Z",
      today_tokens: 99999999,
      health: { label: "健康" },
      delta: { input_tokens: 99, output_tokens: 99, seconds: 2 },
    }),
  );
  assert.equal(
    await page.locator("#today-total").innerText(),
    "1.23M",
    "Stale responses must not overwrite the current key",
  );
  // Restore the preview server's current scope after injecting a late event.
  await page.evaluate(() =>
    window.__previewEmitSample({
      revision: 1,
      sampled_at: "2030-01-01T00:00:02Z",
      today_tokens: 1234567,
      health: { label: "健康" },
      delta: { input_tokens: 0, output_tokens: 0, seconds: 0, baseline: true },
    }),
  );

  // Reopening detail must preserve the shared key scope, not revert the widget to global.
  await page.locator("#widget").click();
  await page.locator(".scope-tag").filter({ hasText: "开发项目" }).waitFor();
  assert.equal(await page.locator("#widget").getAttribute("title"), null);
  assert.equal(
    await page.locator("#widget").evaluate((el) => getComputedStyle(el).cursor),
    "default",
  );
  await page.locator('summary[aria-label="选择 Key owner"]').click();
  await page.locator('[data-pick=key][data-value=""]').click();
  await page.locator("[data-tab=accounts]").waitFor();
  for (const tab of ["analysis", "latency", "distribution", "accounts"]) {
    await page.locator(`[data-tab=${tab}]`).click();
    await page.locator(".skeleton").first().waitFor({ state: "detached" });
    await page.screenshot({ path: `.cache/screenshots/${tab}.png` });
  }
  for (const tab of ["quota-history", "requests", "errors"]) {
    await page.locator(`[data-pick=accountTab][data-value="${tab}"]`).click();
    await page.locator(".skeleton").first().waitFor({ state: "detached" });
    assert.ok(
      await page
        .locator(tab === "errors" ? ".error-event" : ".data-table")
        .count(),
    );
  }
  await page.goto("http://127.0.0.1:1420/?preview=1&theme=dark");
  await page.locator(".metric-value").first().waitFor();
  await page.screenshot({ path: ".cache/screenshots/overview-dark.png" });
  await page.goto("http://127.0.0.1:1420/?preview=1&state=long");
  await page.locator(".metric-value").first().waitFor();
  await page.locator("[data-tab=accounts]").click();
  await page.locator(".skeleton").first().waitFor({ state: "detached" });
  await page.locator("[data-pick=accountTab][data-value=errors]").click();
  await page.locator(".skeleton").first().waitFor({ state: "detached" });
  assert.equal(await page.locator("#content script").count(), 0);
  assert.ok(
    await page
      .locator(".panel-content")
      .evaluate((el) => el.scrollWidth <= el.clientWidth + 1),
  );
  await page.screenshot({ path: ".cache/screenshots/long-error.png" });
  await page.goto("http://127.0.0.1:1420/?preview=1&state=empty");
  await page.locator(".metric-value").first().waitFor();
  assert.equal(await page.locator("#today-total").innerText(), "0");
  await page.locator("[data-tab=latency]").click();
  await page.getByRole("heading", { name: "还没有延迟样本" }).waitFor();
  await page.goto("http://127.0.0.1:1420/?preview=1&state=offline");
  await page.getByRole("heading", { name: "暂时无法读取" }).waitFor();
  await page.screenshot({ path: ".cache/screenshots/offline.png" });
  await page.goto("http://127.0.0.1:1420/?preview=1&window=settings");
  await page.locator("#settings-form").waitFor();
  await page.screenshot({ path: ".cache/screenshots/settings.png" });
  await page.locator("[data-theme=dark]").click();
  assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
  await page
    .locator("#settings-form [name=endpoint]")
    .fill("https://keeper.example/usage");
  const address = page.locator("#settings-form [name=endpoint]");
  await address.press("Control+a");
  assert.equal(
    await address.evaluate((el) => el.selectionEnd - el.selectionStart),
    "https://keeper.example/usage".length,
    "Editable fields keep text selection",
  );
  await page.locator(".proxy-settings summary").click();
  await page.locator("[name=proxyUrl]").fill("socks5h://127.0.0.1:1080");
  assert.equal(
    await page.locator("[name=allowInvalidCertificates]").isChecked(),
    false,
  );
  await page.locator("[name=allowInvalidCertificates]").check();
  await page.screenshot({ path: ".cache/screenshots/tls-warning.png" });
  assert.equal(
    await page.locator("[name=widgetFont]").inputValue(),
    "HarmonyOS Sans SC",
  );
  assert.equal(
    await page.locator("[name=displayHoldSeconds]").inputValue(),
    "16",
  );
  await page.locator("[name=displayHoldSeconds]").fill("6");
  await page.locator("[name=widgetFont]").fill("Microsoft YaHei");
  await page.locator("[name=authMode]").selectOption("api_key");
  assert.equal(
    await page.locator("#credential-label").innerText(),
    "CPA API Key（sk）",
  );
  await page.locator("[name=password]").fill("sk-preview-only");
  await page.locator("[name=password]").press("Control+a");
  assert.equal(
    await page
      .locator("[name=password]")
      .evaluate((el) => el.selectionEnd - el.selectionStart),
    15,
  );
  await page.locator("#save-settings").click();
  await page.getByText("预览模式：未写入注册表。").waitFor();
  assert.equal(
    await page.evaluate(() => window.__previewSavedSettings.value.proxyUrl),
    "socks5h://127.0.0.1:1080",
  );
  assert.equal(
    await page.evaluate(() => window.__previewSavedSettings.value.widgetFont),
    "Microsoft YaHei",
  );
  assert.equal(
    await page.evaluate(() => window.__previewSavedSettings.value.authMode),
    "api_key",
  );
  assert.equal(
    await page.evaluate(
      () => window.__previewSavedSettings.value.allowInvalidCertificates,
    ),
    true,
  );
  assert.equal(
    await page.evaluate(
      () => window.__previewSavedSettings.value.displayHoldSeconds,
    ),
    6,
  );
  await page.evaluate(() => {
    window.__previewSaveError =
      "无法连接 Keeper，请检查地址、代理与网络\n\nERRLOG\ntime=2026-08-27T10:00:00+08:00\nstage=login.request\nroute=proxy socks5h://127.0.0.1:1080\ntarget=https://keeper.example/usage/api/v1/auth/login\ntimeout=false connect=true request=true status=none\ncause[0]=client error (Connect)";
  });
  await page.locator("#save-settings").click();
  await page.getByRole("dialog", { name: "Keeper 连接失败" }).waitFor();
  assert.ok(
    (await page.locator("#connection-errlog").innerText()).includes(
      "stage=login.request",
    ),
  );
  assert.ok(
    (await page.locator("#connection-errlog").innerText()).includes(
      "connect=true",
    ),
  );
  assert.equal(
    await page
      .locator("#connection-errlog")
      .evaluate((el) => el.scrollWidth <= el.clientWidth + 1),
    true,
  );
  await page.screenshot({ path: ".cache/screenshots/connection-errlog.png" });
  await page.locator('[data-error-action="close"]').click();
  assert.equal(await page.locator("#connection-error-dialog").isHidden(), true);
  await page.goto("http://127.0.0.1:1420/?preview=1&role=sk&theme=dark");
  await page.locator(".metric-value").first().waitFor();
  assert.deepEqual(
    await page
      .locator("[data-tab]")
      .evaluateAll((els) => els.map((el) => el.dataset.tab)),
    ["summary"],
  );
  assert.equal(await page.locator("[data-pick=key]").count(), 0);
  assert.equal(await page.locator("#cpa-console").isDisabled(), true);
  assert.ok(
    (await page.locator("#cpa-console").getAttribute("title")).includes(
      "管理员",
    ),
  );
  await page.locator('[data-pick=range][data-value="30d"]').click();
  await page.locator(".scope-tag").filter({ hasText: "近 30 天" }).waitFor();
  assert.ok(
    await page
      .locator(".scope-tag")
      .innerText()
      .then((t) => t.includes("我的项目")),
  );
  assert.ok(
    await page.evaluate(() =>
      window.__previewCalls
        .filter((c) => c.command === "get_view")
        .every((c) => c.args.view === "summary"),
    ),
  );
  await page.screenshot({ path: ".cache/screenshots/sk-overview.png" });
  // Actual native detail size: equal KPI typography and readable contrast in both themes.
  await page.setViewportSize({ width: 640, height: 640 });
  for (const theme of ["light", "dark"]) {
    await page.goto(
      `http://127.0.0.1:1420/?preview=1&standalone&theme=${theme}`,
    );
    await page.locator(".metric-value").first().waitFor();
    await page.evaluate(() => document.fonts.ready);
    const header = await page.evaluate(() => {
      const names = [
        "#connection",
        '[data-console="usage"]',
        '[data-console="cpa"]',
        ".settings-button",
      ];
      const boxes = names.map((selector) =>
        document.querySelector(selector).getBoundingClientRect(),
      );
      const row = document.querySelector(".brand-row");
      return {
        fits: row.scrollWidth <= row.clientWidth + 1,
        ordered: boxes.every((b, i) => i === 0 || b.left >= boxes[i - 1].right),
        width: boxes[3].width,
      };
    });
    assert.ok(
      header.fits && header.ordered && header.width >= 54,
      JSON.stringify(header),
    );

    const metrics = await page.locator(".metric-value").evaluateAll((els) =>
      els.map((el) => ({
        size: getComputedStyle(el).fontSize,
        weight: getComputedStyle(el).fontWeight,
        fits: el.scrollWidth <= el.clientWidth,
      })),
    );
    assert.ok(
      metrics.every((m) => m.size === "26px" && m.weight === "700" && m.fits),
      JSON.stringify(metrics),
    );
    const contrast = await page.evaluate(() => {
      const rgb = (color) => color.match(/\d+/g).slice(0, 3).map(Number);
      const luminance = (color) =>
        rgb(color)
          .map((n) => n / 255)
          .map((n) => (n <= 0.04045 ? n / 12.92 : ((n + 0.055) / 1.055) ** 2.4))
          .reduce((sum, n, i) => sum + n * [0.2126, 0.7152, 0.0722][i], 0);
      return [".metric-label", ".metric-card p", ".metric-row > span"].map(
        (selector) => {
          const el = document.querySelector(selector);
          const fg = luminance(getComputedStyle(el).color);
          const bg = luminance(
            getComputedStyle(el.closest(".metric-card, .rows-card"))
              .backgroundColor,
          );
          return (Math.max(fg, bg) + 0.05) / (Math.min(fg, bg) + 0.05);
        },
      );
    });
    assert.ok(
      contrast.every((ratio) => ratio >= 4.5),
      `${theme}: ${contrast}`,
    );
    await page.locator("[data-tab=accounts]").click();
    const quota = page.locator(".table-wrap").first();
    await quota.waitFor();
    await quota.scrollIntoViewIfNeeded();
    assert.ok(
      await quota.evaluate((el) => el.scrollWidth <= el.clientWidth + 1),
      "Normal quota table must fit without horizontal scrolling",
    );
    const tokens = quota
      .locator("td.cell-number .cell-line")
      .filter({ hasText: "564,617,884" });
    assert.equal(await tokens.innerText(), "564,617,884");
    assert.ok(
      await tokens.evaluate((el) => {
        const r = document.createRange();
        r.selectNodeContents(el);
        return (
          r.getClientRects().length === 1 &&
          r.getBoundingClientRect().width <= el.clientWidth + 1
        );
      }),
      "Quota token count must stay intact on one line",
    );
    await page.screenshot({ path: `.cache/screenshots/quota-${theme}.png` });
    await page.locator('[data-pick=accountTab][data-value="requests"]').click();
    await page.locator(".skeleton").first().waitFor({ state: "detached" });
    assert.ok(
      await page
        .locator(".table-wrap")
        .evaluate((el) => el.scrollWidth > el.clientWidth),
      "Request table scrolls as a unit",
    );
    assert.ok(
      await page
        .locator(".panel-content")
        .evaluate((el) => el.scrollWidth <= el.clientWidth + 1),
    );
  }
  // Controlled preview delivery checks that smoothing affects the widget only.
  await page.goto("http://127.0.0.1:1420/?preview=1&manual");
  await page.locator(".metric-value").first().waitFor();
  const held = await page.evaluate(() => {
    const send = (n, input = 0, output = 0) => {
      window.__previewEmitSample({
        sampled_at: new Date(1700000000000 + n * 2000).toISOString(),
        today_tokens: 295139623,
        health: { label: "健康", failure: 0 },
        delta: {
          input_tokens: input,
          output_tokens: output,
          seconds: 2,
          baseline: false,
          reset: false,
        },
      });
      return [
        document.querySelector("#delta-input").textContent,
        document.querySelector("#delta-output").textContent,
      ];
    };
    send(0, 12480, 1820);
    const retained = [];
    for (let n = 1; n < 8; n++) {
      retained.push(send(n));
      send(n);
    }
    const detail = document.querySelector("#live-summary").textContent;
    const cleared = send(8);
    const clearedDirections = [
      getComputedStyle(document.querySelector("#input-flow svg")).visibility,
      getComputedStyle(document.querySelector("#output-flow svg")).visibility,
    ];
    send(9, 40, 0);
    const oneDirection = [
      getComputedStyle(document.querySelector("#input-flow svg")).visibility,
      getComputedStyle(document.querySelector("#output-flow svg")).visibility,
    ];
    send(10);
    window.__previewEmitError("测试断线");
    return {
      retained,
      detail,
      cleared,
      clearedDirections,
      oneDirection,
      offline: document.querySelector("#delta-input").textContent,
    };
  });
  assert.ok(held.retained.every((pair) => pair.join() === "12.5K,1.82K"));
  assert.ok(held.detail.includes("输入 0 · 输出 0"));
  assert.deepEqual(held.cleared, ["0", "0"]);
  assert.deepEqual(held.clearedDirections, ["hidden", "hidden"]);
  assert.deepEqual(held.oneDirection, ["visible", "hidden"]);
  assert.equal(held.offline, "—");

  // CSS pixel layout at common Windows scale factors; native multi-monitor behavior is a separate manual check.
  for (const scale of [1, 1.25, 1.5, 2]) {
    const context = await browser.newContext({
      viewport: { width: 216, height: 74 },
      deviceScaleFactor: scale,
    });
    try {
      const widgetPage = await context.newPage();
      await widgetPage.goto(
        "http://127.0.0.1:1420/?preview=1&window=widget&standalone&manual",
      );
      await widgetPage.waitForFunction(
        () => document.querySelector("#today-total")?.textContent === "295M",
      );
      await widgetPage.evaluate(() => document.fonts.ready);
      const rect = await widgetPage.locator("#widget").boundingBox();
      assert.equal(rect.width, 200);
      assert.equal(rect.height, 58);
      const fit = await widgetPage.evaluate(() => {
        const widget = document
          .querySelector("#widget")
          .getBoundingClientRect();
        return (
          [
            ...document.querySelectorAll(
              ".widget-number, .widget-unit, .flow-row span, .flow-row strong",
            ),
          ].every((el) => {
            const r = el.getBoundingClientRect();
            return (
              el.scrollWidth <= el.clientWidth + 1 &&
              r.left >= widget.left &&
              r.right <= widget.right &&
              r.top >= widget.top &&
              r.bottom <= widget.bottom
            );
          }) &&
          [...document.querySelectorAll(".flow-row")].every(
            (el) =>
              el.querySelector("strong").getBoundingClientRect().left -
                el.querySelector("span").getBoundingClientRect().right <=
              4,
          )
        );
      });
      assert.ok(fit, `Widget text fits at scale ${scale}`);
      assert.ok(
        !(await widgetPage.locator("#widget").innerText()).match(/今日|内新增/),
      );
      if (scale === 2)
        await widgetPage.screenshot({ path: ".cache/screenshots/widget.png" });
      await widgetPage.evaluate(() =>
        window.__previewEmitSample({
          sampled_at: "2030-01-01T00:00:00Z",
          today_tokens: 8430000,
          health: { label: "波动", failure: 5 },
          delta: { input_tokens: 999999, output_tokens: 999999, seconds: 2 },
        }),
      );
      assert.ok(
        await widgetPage
          .locator(".widget-number, .flow-row strong")
          .evaluateAll((els) =>
            els.every((el) => el.scrollWidth <= el.clientWidth + 1),
          ),
        `Long compact values fit at scale ${scale}`,
      );
    } finally {
      await context.close();
    }
  }
  for (const [side, theme] of [
    ["left", "light"],
    ["right", "dark"],
  ]) {
    const context = await browser.newContext({
      viewport: { width: 34, height: 74 },
    });
    try {
      const edgePage = await context.newPage();
      await edgePage.goto(
        `http://127.0.0.1:1420/?preview=1&window=widget&standalone&manual&edge=${side}&theme=${theme}`,
      );
      await edgePage.locator(".widget-wrap.edge-collapsed").waitFor();
      await edgePage.waitForFunction(
        () =>
          document.querySelector("#edge-token-value")?.textContent === "295",
      );
      const layout = await edgePage.evaluate(() => {
        const wrap = document.querySelector("#widget-wrap");
        const widget = document.querySelector("#widget");
        const value = document.querySelector("#edge-token-value");
        const unit = document.querySelector("#edge-token-unit");
        const health = document.querySelector("#edge-health .dot");
        const rect = (element) => {
          const box = element.getBoundingClientRect();
          return {
            left: box.left,
            top: box.top,
            right: box.right,
            bottom: box.bottom,
            width: box.width,
            height: box.height,
          };
        };
        return {
          edge: wrap.dataset.edge,
          wrap: rect(wrap),
          widget: rect(widget),
          value: rect(value),
          unit: rect(unit),
          health: rect(health),
          valueFits: value.scrollWidth <= value.clientWidth + 1,
          radius: getComputedStyle(widget).borderRadius,
        };
      });
      assert.equal(layout.edge, side);
      assert.equal(layout.wrap.width, 34);
      assert.equal(layout.widget.width, 26);
      assert.equal(layout.widget.height, 58);
      assert.equal(layout.widget.left, side === "left" ? 0 : 8);
      assert.ok(layout.health.bottom <= layout.value.top);
      assert.ok(layout.value.bottom <= layout.unit.top + 1);
      assert.ok(layout.valueFits);
      assert.equal(await edgePage.locator("#edge-token-unit").innerText(), "M");
      assert.equal(
        await edgePage.locator("#edge-health").getAttribute("class"),
        "peek-health green",
      );
      await edgePage.screenshot({
        path: `.cache/screenshots/widget-edge-${side}-${theme}.png`,
      });
    } finally {
      await context.close();
    }
  }
  assert.deepEqual(errors, []);
  console.log(
    "PASS: browser rendering, five tabs, four account views, key/date filters, shared key scope, sk permissions, click entry, themes, empty/offline/long states settings with connection ERRLOG and explicit TLS bypass, native-size and docked-edge layouts, contrast, numeric overflow, DPI rendering, directional arrows and configurable display hold.",
  );
} finally {
  await browser?.close();
  await server.close();
}
