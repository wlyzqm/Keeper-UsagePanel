export function costCoverage(models = []) {
  const used = models.filter(
    (model) =>
      Number(model?.requests || 0) > 0 || Number(model?.total_tokens || 0) > 0,
  );
  const unpriced = used.filter((model) => model?.cost_available !== true);
  return {
    totalModels: used.length,
    pricedModels: used.length - unpriced.length,
    unpricedModels: unpriced
      .map((model) => String(model?.model || model?.label || "").trim())
      .filter(Boolean),
    complete: used.length > 0 && unpriced.length === 0,
  };
}

export function aggregateCostState(source, key, coverage) {
  if (source?.cost_available === true) {
    return { state: "complete", value: source?.[key] ?? 0 };
  }
  if (Number(coverage?.pricedModels || 0) > 0) {
    return { state: "partial", value: source?.[key] ?? 0 };
  }
  return { state: "missing", value: null };
}
