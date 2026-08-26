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
  await page.locator(".hero-value").first().waitFor();
  await page.screenshot({ path: ".cache/screenshots/overview-light.png" });
  assert.equal(await page.locator("#today-total").innerText(), "8.43M");
  assert.equal(await page.locator("#delta-input").innerText(), "12.5K");
  await page.locator('[data-pick=range][data-value="30d"]').click();
  await page.locator(".scope-tag").filter({ hasText: "近 30 天" }).waitFor();
  await page.locator('summary[aria-label="按 Key 筛选"]').click();
  await page.locator("[data-pick=key][data-value=k1]").click();
  await page.locator(".scope-tag").filter({ hasText: "开发项目" }).waitFor();
  assert.equal(await page.locator("#today-total").innerText(), "8.43M");
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
  await page.locator(".hero-value").first().waitFor();
  await page.screenshot({ path: ".cache/screenshots/overview-dark.png" });
  await page.goto("http://127.0.0.1:1420/?preview=1&state=long");
  await page.locator(".hero-value").first().waitFor();
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
  await page.locator(".hero-value").first().waitFor();
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
  await page.locator(".proxy-settings summary").click();
  await page.locator("[name=proxyUrl]").fill("socks5h://127.0.0.1:1080");
  assert.equal(
    await page.locator("[name=widgetFont]").inputValue(),
    "HarmonyOS Sans SC",
  );
  await page.locator("[name=widgetFont]").fill("Microsoft YaHei");
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
  assert.deepEqual(errors, []);
  console.log(
    "PASS: browser rendering, five tabs, four account views, key/date filters, global widget, themes, empty/offline/long states and settings.",
  );
} finally {
  await browser?.close();
  await server.close();
}
