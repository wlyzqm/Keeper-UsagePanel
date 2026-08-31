// Development only. Vite removes this import from the production bundle.
export function createPreview(search) {
  const listeners = new Map();
  let edgeStateCalls = 0;
  const viewer = search.get("role") === "sk";
  let access = {
    role: viewer ? "api_key_viewer" : "admin",
    api_key: { alias: "我的项目", display_key: "sk-***123456" },
    scope: { api_key_id: "", label: "全部 Key", revision: 0 },
  };
  window.__previewCalls = [];
  window.__previewSaveError = "";
  window.__previewEvent = (name, value) => emit(name, value);
  const emit = (name, value) =>
    (listeners.get(name) || []).forEach((fn) => fn(value));
  const empty = search.get("state") === "empty",
    offline = search.get("state") === "offline",
    long = search.get("state") === "long",
    costMode = search.get("cost") || "complete";
  const stats = {
    total_tokens: empty ? 0 : 295139623,
    total_requests: empty ? 0 : 2493,
    success_count: empty ? 0 : 2408,
    failure_count: empty ? 0 : 85,
  };
  const activity = {
    input_tokens: empty ? 0 : 293553727,
    output_tokens: empty ? 0 : 1585896,
    cache_read_tokens: empty ? 0 : 275625728,
    cache_creation_tokens: empty ? 0 : 0,
    reasoning_tokens: empty ? 0 : 592429,
  };
  let sample = {
    sampled_at: new Date().toISOString(),
    today_tokens: stats.total_tokens,
    delta: {
      input_tokens: empty ? 0 : 12480,
      output_tokens: empty ? 0 : 1820,
      seconds: 2,
      baseline: false,
      reset: false,
    },
    health: {
      label: "健康",
      success: 324,
      failure: 3,
      basis: viewer ? "key_requests" : "credentials",
    },
  };
  window.__previewEmitSample = (next) => {
    sample = next;
    emit("sample", sample);
  };
  window.__previewEmitError = (error) => emit("connection-error", error);
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
    credential_health: {
      total_success: 324,
      total_failure: 3,
      buckets: Array.from({ length: 30 }, (_, index) => ({
        success: index < 27 ? 10 : 0,
        failure: index === 27 ? 1 : 0,
      })),
    },
  };
  const secondAccount = {
    ...account,
    id: "account-2",
    identity: "auth-index-2",
    name: "Team",
    displayName: "团队账户 · Spark",
    total_requests: 1184,
    total_tokens: 126439082,
    success_count: 1178,
    failure_count: 6,
    input_tokens: 118529330,
    cache_read_tokens: 103452910,
    credential_health: {
      total_success: 298,
      total_failure: 2,
      buckets: Array.from({ length: 30 }, () => ({ success: 10, failure: 0 })),
    },
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
      cost_available: costMode !== "none",
    },
    {
      label: "gpt-5.4-mini",
      total_tokens: 1580870,
      percent: 18.8,
      requests: 284,
      cost_usd: 1.702,
      cost_available: costMode === "complete",
    },
  ];
  const analysis = {
    cost_breakdown: {
      cost_available: costMode === "complete",
      total_cost_usd:
        costMode === "none" ? 0 : costMode === "partial" ? 16.923 : 42.8965,
      uncached_input_cost_usd: costMode === "none" ? 0 : 5.24,
      cache_read_cost_usd: costMode === "none" ? 0 : 1.68,
      cache_write_cost_usd: costMode === "none" ? 0 : 0.44,
      output_cost_usd: costMode === "none" ? 0 : 9.563,
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
      window.__previewCalls.push({ command, args });
      if (command === "open_console") return;
      if (command === "widget_edge_state") {
        edgeStateCalls += 1;
        if (search.has("edgeMissed"))
          return edgeStateCalls === 1
            ? { side: null, collapsed: false, ready: false }
            : { side: "right", collapsed: true, ready: true };
        if (search.has("edgeRace")) {
          setTimeout(
            () =>
              emit("widget-edge", {
                side: "right",
                collapsed: true,
                ready: true,
              }),
            25,
          );
          await new Promise((resolve) => setTimeout(resolve, 100));
          return { side: null, collapsed: false, ready: false };
        }
        const edgeDelay = Math.min(
          1000,
          Math.max(0, Number(search.get("edgeDelay")) || 0),
        );
        if (edgeDelay)
          await new Promise((resolve) => setTimeout(resolve, edgeDelay));
        const side = ["left", "right"].includes(search.get("edge"))
          ? search.get("edge")
          : null;
        return {
          side,
          collapsed: !!side && search.get("expanded") !== "1",
          ready: true,
        };
      }
      if (command === "get_access") return structuredClone(access);
      if (command === "set_scope") {
        if (viewer) throw "sk 登录不能切换 Key owner";
        access.scope = {
          api_key_id: args.apiKeyId,
          label: args.apiKeyId ? "开发项目" : "全部 Key",
          revision: access.scope.revision + 1,
        };
        sample = {
          ...sample,
          revision: access.scope.revision,
          sampled_at: new Date().toISOString(),
          today_tokens: args.apiKeyId ? 1234567 : stats.total_tokens,
          delta: {
            input_tokens: 0,
            output_tokens: 0,
            seconds: 0,
            baseline: true,
            reset: false,
          },
        };
        emit("scope-changed", structuredClone(access));
        return structuredClone(access);
      }
      if (command === "get_settings")
        return {
          endpoint: "https://keeper.example/usage",
          authMode: viewer ? "api_key" : "admin",
          pollSeconds: 2,
          displayHoldSeconds: 16,
          theme: search.get("theme") || "light",
          accentColor: "",
          rememberPassword: true,
          hasPassword: false,
          autoStart: false,
          allowPrivateHttp: false,
          allowInvalidCertificates: false,
          edgeAutoCollapse: true,
          fullscreenAutoHide: true,
          skippedUpdateVersion: "",
        };
      if (command === "pending_update")
        return search.has("update")
          ? {
              version: "0.6.0",
              notes:
                "- 新增自动更新\n- 优化便携版原位替换\n- 更新下载沿用代理与证书设置",
              releaseUrl:
                "https://github.com/wlyzqm/Keeper-UsagePanel/releases/tag/v0.6.0",
              portable: search.get("installed") !== "1",
            }
          : null;
      if (command === "check_update") {
        const update = search.has("update")
          ? {
              version: "0.6.0",
              notes: "- 新增自动更新\n- 优化便携版原位替换",
              releaseUrl:
                "https://github.com/wlyzqm/Keeper-UsagePanel/releases/tag/v0.6.0",
              portable: search.get("installed") !== "1",
            }
          : null;
        emit("update-status", update);
        return update;
      }
      if (["skip_update", "install_update"].includes(command)) {
        window.__previewUpdateAction = command;
        if (command === "skip_update") emit("update-status", null);
        return;
      }
      if (command === "last_sample") return offline ? null : sample;
      if (command === "sample") {
        if (offline) throw "无法连接 Keeper，请检查地址与网络";
        if (!search.has("manual")) sample.sampled_at = new Date().toISOString();
        emit("sample", sample);
        return sample;
      }
      if (command === "save_settings") {
        if (window.__previewSaveError) throw window.__previewSaveError;
        window.__previewSavedSettings = args;
        return;
      }
      if (command === "window_action") {
        if (args.action === "detail") emit("detail-open");
        if (args.action === "close-detail") emit("detail-close");
        if (args.action === "drag") {
          await new Promise((resolve) => setTimeout(resolve, 100));
          emit("drag-finished");
        }
        return;
      }
      if (command !== "get_view") throw new Error("Unknown preview command");
      if (viewer && args.view !== "summary") throw "sk 登录无权访问此指标";
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
              usage: access.scope.api_key_id
                ? { ...stats, total_tokens: 1234567 }
                : stats,
              summary: {
                ...activity,
                total_cost:
                  empty || costMode === "none"
                    ? 0
                    : costMode === "partial"
                      ? 16.923
                      : 42.8965,
                cost_available: costMode === "complete",
              },
            },
            activity,
            cost_coverage: {
              total_models: empty ? 0 : 2,
              priced_models:
                empty || costMode === "none"
                  ? 0
                  : costMode === "partial"
                    ? 1
                    : 2,
              unpriced_models:
                costMode === "none"
                  ? composition.map((model) => model.label)
                  : costMode === "partial"
                    ? [composition[1].label]
                    : [],
              complete: costMode === "complete",
            },
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
          return {
            identities: empty ? [] : [account, secondAccount],
            total_count: empty ? 0 : 2,
            quota_items: empty
              ? []
              : [account, secondAccount].map((entry, index) => ({
                  auth_index: entry.identity,
                  refreshed_at: sample.sampled_at,
                  quota: {
                    subscription: { plan: index ? "Pro 20x" : "Plus" },
                    quota: index
                      ? [
                          {
                            label: "GPT-5.3-Codex-Spark-5h",
                            remainingFraction: 0.51,
                            resetAt: sample.sampled_at,
                          },
                          {
                            label: "GPT-5.3-Codex-Spark-Weekly",
                            usedPercent: 64,
                            resetAt: sample.sampled_at,
                          },
                          {
                            label: "Weekly",
                            usedPercent: 100,
                            resetAt: sample.sampled_at,
                          },
                        ]
                      : [
                          {
                            label: "GPT-5.3-Codex-5h",
                            remainingFraction: 0.82,
                            resetAt: sample.sampled_at,
                          },
                          {
                            label: "GPT-5.3-Codex-Spark-5h",
                            usedPercent: 34,
                            resetAt: sample.sampled_at,
                          },
                          {
                            label: "Weekly",
                            usedPercent: 28,
                            resetAt: sample.sampled_at,
                          },
                          {
                            label: "GPT-5.3-Codex-Spark-Weekly",
                            usedPercent: 7,
                            resetAt: sample.sampled_at,
                          },
                        ],
                  },
                })),
          };
        case "quota":
          return {
            items: [
              {
                refreshed_at: sample.sampled_at,
                quota: {
                  subscription: { plan: "Plus" },
                  quota: [
                    {
                      label: "GPT-5.3-Codex-5h",
                      remainingFraction: 0.82,
                      resetAt: sample.sampled_at,
                      window_usage_tokens: 972610,
                      window_usage_cost: 2.71,
                    },
                    {
                      label: "GPT-5.3-Codex-Spark-5h",
                      usedPercent: 34,
                      resetAt: sample.sampled_at,
                      window_usage_tokens: 1408230,
                      window_usage_cost: 4.38,
                    },
                    {
                      label: "Weekly",
                      usedPercent: 28,
                      window_usage_tokens: 564617884,
                      window_usage_cost: 89.9891,
                      resetAt: sample.sampled_at,
                    },
                    {
                      label: "GPT-5.3-Codex-Spark-Weekly",
                      usedPercent: 7,
                      window_usage_tokens: 8512741,
                      window_usage_cost: 1.92,
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
            cycles: Array.from(
              { length: search.get("state") === "resets" ? 8 : 1 },
              (_, index) => ({
                status: index ? "ended" : "current",
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
                    percentage_points: 18,
                    tokens_per_point: 78235,
                    cost_per_point: 0.2433333333,
                    cost_per_point_available: true,
                    usage: {
                      total_tokens: 1408230,
                      total_cost_usd: 4.38,
                      cost_available: true,
                    },
                  },
                ],
              }),
            ),
          };
        case "requests":
          return {
            events: Array.from({ length: q.cursor ? 2 : 24 }, (_, i) => ({
              timestamp: sample.sampled_at,
              api_key:
                i % 3 === 0
                  ? "sk-*********123456"
                  : i % 2
                    ? "日常使用"
                    : "开发项目",
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
