import test from "node:test";
import assert from "node:assert/strict";
import { aggregateCostState, costCoverage } from "../src/cost-state.js";

test("cost coverage distinguishes complete, partial, and missing pricing", () => {
  const complete = costCoverage([
    { model: "priced-a", requests: 1, total_tokens: 10, cost_available: true },
    { model: "priced-b", requests: 1, total_tokens: 20, cost_available: true },
  ]);
  assert.deepEqual(complete, {
    totalModels: 2,
    pricedModels: 2,
    unpricedModels: [],
    complete: true,
  });

  const partial = costCoverage([
    { model: "priced", requests: 1, total_tokens: 10, cost_available: true },
    { model: "unpriced", requests: 1, total_tokens: 20, cost_available: false },
  ]);
  assert.deepEqual(partial, {
    totalModels: 2,
    pricedModels: 1,
    unpricedModels: ["unpriced"],
    complete: false,
  });
  assert.deepEqual(
    aggregateCostState(
      { total_cost_usd: 1, cost_available: false },
      "total_cost_usd",
      partial,
    ),
    { state: "partial", value: 1 },
  );

  const missing = costCoverage([
    { model: "unpriced", requests: 1, total_tokens: 20, cost_available: false },
  ]);
  assert.deepEqual(
    aggregateCostState(
      { total_cost_usd: 0, cost_available: false },
      "total_cost_usd",
      missing,
    ),
    { state: "missing", value: null },
  );
});

test("unused unpriced models do not reduce pricing coverage", () => {
  assert.deepEqual(
    costCoverage([
      { model: "priced", requests: 1, total_tokens: 10, cost_available: true },
      { model: "unused", requests: 0, total_tokens: 0, cost_available: false },
    ]),
    {
      totalModels: 1,
      pricedModels: 1,
      unpricedModels: [],
      complete: true,
    },
  );
});
