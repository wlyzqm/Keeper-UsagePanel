// Development only. Vite removes this import from the production bundle.
export function createPreview(search) {
  const listeners = new Map();
  const emit = (name, value) =>
    (listeners.get(name) || []).forEach((fn) => fn(value));
  const empty = search.get("state") === "empty",
    offline = search.get("state") === "offline",
    long = search.get("state") === "long";
  const stats = {
    total_tokens: empty ? 0 : 8426190,
    total_requests: empty ? 0 : 1248,
    success_count: empty ? 0 : 1239,
    failure_count: empty ? 0 : 9,
  };
  const activity = {
    input_tokens: empty ? 0 : 7182430,
    output_tokens: empty ? 0 : 1243760,
    cache_read_tokens: empty ? 0 : 5679308,
    cache_creation_tokens: empty ? 0 : 148240,
    reasoning_tokens: empty ? 0 : 628510,
  };
  const sample = {
    sampled_at: new Date().toISOString(),
    today_tokens: stats.total_tokens,
    delta: {
      input_tokens: empty ? 0 : 12480,
      output_tokens: empty ? 0 : 1820,
      seconds: 2,
      baseline: false,
      reset: false,
    },
    health: { label: "健康", success: 324, failure: 3 },
  };
  const account = {
    id: "account-1",
    identity: "auth-index-1",
    name: "Personal",
    displayName: long
      ? "这是一个很长的账户名称，用于验证界面布局和溢出，不包含任何真实账户信息"
      : "个人账户 · Codex",
    provider: "OpenAI",
    type: "codex",
    ...stats,
    ...activity,
    last_used_at: sample.sampled_at,
    credential_health: { total_success: 324, total_failure: 3 },
  };
  const composition = [
    {
      label: long
        ? "a-very-long-model-name-to-check-the-table-layout-and-wrapping-without-overflow"
        : "gpt-5.4",
      total_tokens: 6845320,
      percent: 81.2,
      requests: 964,
      cost_usd: 16.923,
      cost_available: true,
    },
    {
      label: "gpt-5.4-mini",
      total_tokens: 1580870,
      percent: 18.8,
      requests: 284,
      cost_usd: 1.702,
      cost_available: true,
    },
  ];
  const analysis = {
    cost_breakdown: {
      cost_available: true,
      total_cost_usd: 18.625,
      uncached_input_cost_usd: 5.24,
      cache_read_cost_usd: 1.68,
      cache_write_cost_usd: 0.44,
      output_cost_usd: 11.265,
    },
    model_efficiency: composition.map((m) => ({
      ...m,
      model: m.label,
      cost_per_request_usd: 0.018,
      output_tokens_per_request: 998,
      input_tokens: 5000,
      cache_read_tokens: 3900,
    })),
    model_composition: composition,
    api_key_composition: composition.map((m, i) => ({
      ...m,
      label: i ? "日常使用" : "开发项目",
    })),
    auth_files_composition: [{ ...composition[0], label: "个人账户 · Codex" }],
    ai_provider_composition: [{ ...composition[0], label: "OpenAI" }],
  };
  return {
    on: async (name, fn) => {
      listeners.set(name, [...(listeners.get(name) || []), fn]);
      return () => {};
    },
    call: async (command, args = {}) => {
      if (command === "get_settings")
        return {
          endpoint: "https://keeper.example/usage",
          pollSeconds: 2,
          theme: search.get("theme") || "light",
          rememberPassword: true,
          hasPassword: false,
          autoStart: false,
          allowPrivateHttp: false,
        };
      if (command === "last_sample") return offline ? null : sample;
      if (command === "sample") {
        if (offline) throw "无法连接 Keeper，请检查地址与网络";
        sample.sampled_at = new Date().toISOString();
        emit("sample", sample);
        return sample;
      }
      if (command === "save_settings") {
        window.__previewSavedSettings = args;
        return;
      }
      if (command === "window_action") {
        if (args.action === "detail") emit("detail-open");
        return;
      }
      if (command !== "get_view") throw new Error("Unknown preview command");
      await new Promise((resolve) => setTimeout(resolve, 80));
      if (offline) throw "连接超时，请检查 Keeper 地址后重试";
      const q = args.query || {};
      switch (args.view) {
        case "keys":
          return {
            options: [
              { id: "k1", label: "开发项目" },
              {
                id: "k2",
                label: long
                  ? "这是一个非常长的 Key 名称：用于验证长文本截断与选择器展示"
                  : "日常使用",
              },
            ],
          };
        case "summary":
          return {
            overview: {
              usage: stats,
              summary: {
                ...activity,
                total_cost: empty ? 0 : 18.625,
                cost_available: true,
              },
            },
            activity,
          };
        case "analysis":
          return empty ? { cost_breakdown: {} } : analysis;
        case "latency":
          return empty
            ? { total_points: 0 }
            : {
                total_points: 942,
                p95_ttft_ms: 1820,
                p95_latency_ms: 28450,
                max_ttft_ms: 4310,
                max_latency_ms: 96200,
              };
        case "accounts":
          return { identities: empty ? [] : [account], total_pages: 1 };
        case "quota":
          return {
            items: [
              {
                refreshed_at: sample.sampled_at,
                quota: {
                  subscription: { plan: "Plus" },
                  quota: [
                    {
                      label: "5 小时",
                      remainingFraction: 0.82,
                      resetAt: sample.sampled_at,
                      window_usage_tokens: 1408230,
                      window_usage_cost: 4.38,
                    },
                    {
                      label: "每周",
                      remainingFraction: 0.67,
                      resetAt: sample.sampled_at,
                    },
                  ],
                },
              },
            ],
          };
        case "quota-history":
          return {
            supported: true,
            cycles: [
              {
                status: "current",
                window_started_at: sample.sampled_at,
                reset_at: sample.sampled_at,
                first_remaining_percent: 100,
                last_remaining_percent: 82,
                usage: {
                  requests: 324,
                  total_tokens: 1408230,
                  total_cost_usd: 4.38,
                  cost_available: true,
                },
                transitions: [
                  {
                    interval_ended_at: sample.sampled_at,
                    from_remaining_percent: 100,
                    to_remaining_percent: 82,
                    tokens_per_point: 78235,
                    cost_per_point: 0.24,
                    cost_per_point_available: true,
                    usage: { total_tokens: 1408230 },
                  },
                ],
              },
            ],
          };
        case "requests":
          return {
            events: Array.from({ length: q.cursor ? 2 : 6 }, (_, i) => ({
              timestamp: sample.sampled_at,
              model: i % 2 ? "gpt-5.4-mini" : "gpt-5.4",
              failed: false,
              tokens: {
                input_tokens: 12480,
                output_tokens: 1820,
                cache_read_tokens: 9200,
                reasoning_tokens: 812,
                total_tokens: 14300,
              },
              cost_usd: 0.04,
              cost_available: true,
              ttft_ms: 680,
              latency_ms: 3120,
            })),
            has_more: !q.cursor,
            next_cursor: "next",
          };
        case "errors":
          return {
            events: [
              {
                timestamp: sample.sampled_at,
                model: "gpt-5.4",
                status_code: 429,
                code: "rate_limit_exceeded",
                body_summary: long
                  ? "A very long error message <script>alert(1)</script> & diagnostic information. ".repeat(
                      5,
                    )
                  : "请求频率超过当前额度限制",
                retryable: true,
              },
            ],
            total_count: 1,
            has_more: false,
          };
        default:
          throw new Error("Unknown preview view");
      }
    },
  };
}
