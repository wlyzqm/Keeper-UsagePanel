import test from "node:test";
import assert from "node:assert/strict";
import {
  compact,
  edgeCompact,
  number,
  percent,
  duration,
  cost,
  escape,
  time,
  widgetFontStack,
} from "../src/format.js";
test("compact numbers preserve trailing integer zeroes", () => {
  assert.equal(compact(100000), "100K");
  assert.equal(compact(10000), "10K");
  assert.equal(compact(8426190), "8.43M");
  assert.equal(compact(5646178840000), "5.65T");
  assert.equal(compact(1234000000000000), "1.23P");
  assert.equal(compact(1234000000000000000), "1.23E");
  assert.equal(compact(0), "0");
  assert.equal(compact(null), "—");
});
test("missing values are not invented zeros", () => {
  assert.equal(number(null), "—");
  assert.equal(percent(0, 0), "—");
  assert.equal(duration(0), "—");
  assert.equal(cost({ cost_usd: 0 }), "—");
  assert.equal(cost({ cost_usd: 0, cost_available: true }), "$0.00");
});
test("untrusted Keeper labels are escaped", () => {
  assert.equal(escape("<script>\"&'"), "&lt;script&gt;&quot;&amp;&#39;");
});
test("timestamps always use Beijing", () => {
  assert.equal(time("2026-08-26T00:00:00Z"), "08/26 08:00:00");
});
test("widget defaults to HarmonyOS and preserves Chinese fallbacks", () => {
  assert.ok(widgetFontStack().startsWith('"HarmonyOS Sans SC"'));
  assert.ok(
    widgetFontStack("Example Font").startsWith(
      '"Example Font","HarmonyOS Sans SC"',
    ),
  );
  assert.ok(widgetFontStack().includes('"Microsoft YaHei"'));
});

test("edge compact total stays within three glyphs and separates magnitude", () => {
  assert.deepEqual(edgeCompact(null), { value: "—", unit: "" });
  assert.deepEqual(edgeCompact(842), { value: "842", unit: "" });
  assert.deepEqual(edgeCompact(1234567), { value: "1.2", unit: "M" });
  assert.deepEqual(edgeCompact(295139623), { value: "295", unit: "M" });
  assert.deepEqual(edgeCompact(999900), { value: "1", unit: "M" });
});
