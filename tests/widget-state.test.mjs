import assert from "node:assert/strict";
import test from "node:test";
import { WidgetDeltaDisplay } from "../src/widget-state.js";
const sample = (n, input = 0, output = 0, seconds = 2, flags = {}) => ({
  sampled_at: new Date(1700000000000 + n * 1000).toISOString(),
  delta: { input_tokens: input, output_tokens: output, seconds, ...flags },
});
test("hold both values until 16 seconds of consecutive zero samples", () => {
  const display = new WidgetDeltaDisplay();
  display.update(sample(0, 12480, 1820));
  for (let n = 1; n < 8; n++) {
    const s = sample(n);
    assert.deepEqual(display.update(s), {
      input: 12480,
      output: 1820,
      held: true,
    });
    assert.deepEqual(display.update(s), {
      input: 12480,
      output: 1820,
      held: true,
    });
  }
  assert.deepEqual(display.update(sample(8)), {
    input: 0,
    output: 0,
    held: false,
  });
});
test("new usage in either direction replaces the pair and restarts the idle timer", () => {
  const display = new WidgetDeltaDisplay();
  display.update(sample(0, 100, 200));
  display.update(sample(1, 0, 0, 15));
  assert.deepEqual(display.update(sample(2, 0, 42)), {
    input: 0,
    output: 42,
    held: false,
  });
  assert.deepEqual(display.update(sample(3, 0, 0, 5)), {
    input: 0,
    output: 42,
    held: true,
  });
  assert.deepEqual(display.update(sample(4, 0, 0, 11)), {
    input: 0,
    output: 0,
    held: false,
  });
  assert.deepEqual(display.update(sample(5, 9, 0)), {
    input: 9,
    output: 0,
    held: false,
  });
});
test("disconnect, reconfigure, baseline and reset clear the displayed history", () => {
  for (const reset of [
    null,
    sample(2, 0, 0, 2, { baseline: true }),
    sample(2, 0, 0, 2, { reset: true }),
    "offline",
  ]) {
    const display = new WidgetDeltaDisplay();
    const current = sample(1, 100, 200);
    display.update(current);
    assert.deepEqual(
      display.update(
        reset === "offline" ? current : reset,
        reset === "offline",
      ),
      { input: null, output: null, held: false },
    );
    assert.deepEqual(display.update(sample(3)), {
      input: 0,
      output: 0,
      held: false,
    });
  }
});
test("first zero sample displays zero; long polling intervals use elapsed seconds", () => {
  const display = new WidgetDeltaDisplay();
  assert.deepEqual(display.update(sample(0)), {
    input: 0,
    output: 0,
    held: false,
  });
  display.update(sample(1, 100, 200));
  assert.deepEqual(display.update(sample(2, 0, 0, 60)), {
    input: 0,
    output: 0,
    held: false,
  });
});
