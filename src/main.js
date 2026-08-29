import "./style.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { WidgetDeltaDisplay } from "./widget-state.js";
import { icon } from "./icons.js";
import {
  escape as e,
  number,
  compact,
  edgeCompact,
  percent,
  duration,
  money,
  cost,
  time,
  day,
  tone,
  widgetFontStack,
} from "./format.js";

const search = new URLSearchParams(location.search);
const preview = import.meta.env.DEV && search.get("preview") === "1";
const windowName = search.get("window") || "detail";
let api = {
  call: invoke,
  on: async (name, fn) => listen(name, (event) => fn(event.payload)),
};
if (preview) api = (await import("./preview.js")).createPreview(search);
const widgetDelta = new WidgetDeltaDisplay();
const state = {
  settings: { pollSeconds: 2, displayHoldSeconds: 16, theme: "light" },
  sample: null,
  access: null,
  revision: 0,
  epoch: 0,
  error: "",
  tab: "summary",
  range: "today",
  key: "",
  keys: [],
  start: day(),
  end: day(),
  distribution: "model_composition",
  accounts: [],
  account: "",
  accountPage: 1,
  accountPages: 1,
  accountTab: "quota",
  cursor: "",
  role: "primary",
  generation: 0,
  visible: windowName === "detail",
  data: null,
  loading: false,
};
const tabs = [
  ["summary", "总览", "overview"],
  ["analysis", "成本", "cost"],
  ["latency", "延迟", "latency"],
  ["distribution", "分布", "distribution"],
  ["accounts", "账户", "accounts"],
];
const availableTabs = () =>
  !state.access
    ? []
    : state.access.role === "api_key_viewer"
      ? tabs.filter(([id]) => id === "summary")
      : tabs.filter(([id]) => id !== "accounts" || !state.key);
const scopeLabel = () =>
  state.access?.role === "api_key_viewer"
    ? state.access.api_key?.alias ||
      state.access.api_key?.display_key ||
      "当前 sk"
    : state.access?.scope?.label || "全部 Key";
const viewCall = (view, query = {}) =>
  api.call("get_view", { view, query, revision: state.revision });
const ranges = {
  today: "今日",
  yesterday: "昨日",
  "7d": "近 7 天",
  "30d": "近 30 天",
  month: "本月",
  custom: "自定义",
};
const accountTabs = [
  ["quota", "概览"],
  ["quota-history", "额度历史"],
  ["requests", "请求明细"],
  ["errors", "错误事件"],
];
const $ = (selector) => document.querySelector(selector);
const root = $("#app");
const applyAppearance = () => {
  document.documentElement.dataset.theme = state.settings.theme || "light";
  const accent = /^#[0-9a-f]{6}$/i.test(state.settings.accentColor || "")
    ? state.settings.accentColor
    : "";
  document.documentElement.toggleAttribute("data-custom-accent", !!accent);
  if (accent)
    document.documentElement.style.setProperty("--accent-custom", accent);
  else document.documentElement.style.removeProperty("--accent-custom");
  document.documentElement.style.setProperty(
    "--widget-font",
    widgetFontStack(state.settings.widgetFont),
  );
};
const applySettings = () => {
  applyAppearance();
  widgetDelta.setIdleSeconds(state.settings.displayHoldSeconds ?? 16);
};
const action = async (name) => {
  try {
    await api.call("window_action", { action: name });
    return true;
  } catch (error) {
    console.error(error);
    return false;
  }
};
const parseConnectionError = (error) => {
  const raw =
    typeof error === "string"
      ? error
      : error?.message || JSON.stringify(error) || "未知连接错误";
  const marker = "\n\nERRLOG\n";
  const split = raw.indexOf(marker);
  return split < 0
    ? {
        message: raw.trim() || "无法连接 Keeper",
        errlog: `stage=settings.connect\ndetail=${raw.trim() || "unknown error"}`,
      }
    : {
        message: raw.slice(0, split).trim() || "无法连接 Keeper",
        errlog: raw.slice(split + marker.length).trim(),
      };
};
const showConnectionError = (error) => {
  const diagnostic = parseConnectionError(error);
  $("#settings-error").textContent = diagnostic.message;
  $("#connection-error-summary").textContent = diagnostic.message;
  $("#connection-errlog").textContent = diagnostic.errlog;
  const dialog = $("#connection-error-dialog");
  dialog.hidden = false;
  dialog.querySelector('[data-error-action="copy"]')?.focus();
};
const closeConnectionError = () => {
  const dialog = $("#connection-error-dialog");
  if (!dialog) return;
  dialog.hidden = true;
  $("#save-settings")?.focus();
};
const copyConnectionErrlog = async (button) => {
  const value = $("#connection-errlog")?.textContent || "";
  try {
    await navigator.clipboard.writeText(value);
  } catch {
    const field = document.createElement("textarea");
    field.value = value;
    field.setAttribute("readonly", "");
    field.className = "clipboard-field";
    document.body.append(field);
    field.select();
    document.execCommand("copy");
    field.remove();
  }
  button.textContent = "已复制 ERRLOG";
};
const button = (name, title) =>
  `<button class="icon-button" data-action="${name}" title="${title}" aria-label="${title}">${icon(name === "close-detail" || name === "close-settings" ? "close" : name)}</button>`;
const note = (text) =>
  `<div class="note">${icon("info")}<p>${e(text)}</p></div>`;
const rows = (items) =>
  `<div class="rows-card">${items.map(([label, value]) => `<div class="metric-row"><span>${e(label)}</span><strong class="num">${e(value)}</strong></div>`).join("")}</div>`;
const cards = (items, columns = 2) =>
  `<div class="metric-grid columns-${columns}">${items.map(([label, value, hint, mark]) => `<div class="metric-card"><div class="metric-label">${mark ? icon(mark) : ""}${e(label)}</div><div class="metric-value num" title="${e(value)}">${e(value)}</div>${hint ? `<p title="${e(hint)}">${e(hint)}</p>` : ""}</div>`).join("")}</div>`;
const heading = (text, right = "") =>
  `<div class="group-title"><span>${e(text)}</span><span class="muted">${e(right)}</span></div>`;
const table = (headers, items, kinds = [], extraClass = "") =>
  items.length
    ? `<div class="table-wrap ${extraClass}" tabindex="0" role="region" aria-label="${e(headers[0])}明细表，可横向滚动"><table class="data-table"><thead><tr>${headers.map((h, i) => `<th scope="col" class="cell-${kinds[i] || (i ? "number" : "name")}">${e(h)}</th>`).join("")}</tr></thead><tbody>${items
        .map(
          (row) =>
            `<tr>${row
              .map((v, i) => {
                const kind = kinds[i] || (i ? "number" : "name");
                const lines = Array.isArray(v) ? v : [v ?? "—"];
                return `<td class="cell-${kind}"><span class="cell-content" title="${e(lines.join(" / "))}">${lines.map((line) => `<span class="cell-line">${e(line)}</span>`).join("")}</span></td>`;
              })
              .join("")}</tr>`,
        )
        .join("")}</tbody></table></div>`
    : `<div class="rows-card empty-inline">此范围暂无记录</div>`;
const requestTable = (headers, items, kinds) =>
  items.length
    ? `<div class="request-table-frame"><div class="request-table-scroll" tabindex="0" role="region" aria-label="请求明细横向滚动条"><div class="request-table-scroll-width"></div></div>${table(headers, items, kinds, "request-table-wrap")}</div>`
    : `<div class="rows-card empty-inline">此范围暂无记录</div>`;
const picker = (name, label, options, selected, extra = "") =>
  `<details class="picker ${extra}"><summary aria-label="${name === "key" ? "选择 Key owner" : name === "range" ? "更多日期范围" : name === "account" ? "选择认证账户" : "切换分布维度"}">${name === "key" ? icon("key") : ""}<span class="picker-label">${e(label)}</span>${icon("chevron")}</summary><div class="picker-menu">${options.map(([value, text]) => `<button data-pick="${name}" data-value="${e(value)}" class="${String(value) === String(selected) ? "selected" : ""}" title="${e(text)}">${e(text)}</button>`).join("")}</div></details>`;

function widget() {
  return `<div class="widget-wrap" id="widget-wrap"><div class="widget" id="widget" role="button" tabindex="0" aria-label="Keeper 用量，悬停或点击查看详情，拖动移动"><span class="widget-health neutral" id="health" role="img" aria-label="未连接" title="未连接"><i class="dot"></i></span><div class="widget-total"><strong class="widget-number num" id="today-total">—</strong><span class="widget-unit">Token</span></div><div class="widget-flows"><div class="flow-row flow-zero" id="input-flow">${icon("output")}<span>输入</span><strong class="num" id="delta-input">—</strong></div><div class="flow-row flow-zero" id="output-flow">${icon("input")}<span>输出</span><strong class="num" id="delta-output">—</strong></div></div><div class="widget-peek" aria-hidden="true"><span class="peek-health neutral" id="edge-health"><i class="dot"></i></span><span class="peek-total num"><strong id="edge-token-value">—</strong><span id="edge-token-unit"></span></span></div></div></div>`;
}

function applyWidgetEdge(next = {}) {
  const wrap = $("#widget-wrap");
  if (!wrap) return;
  const side = next.side === "left" || next.side === "right" ? next.side : "";
  wrap.classList.toggle("edge-collapsed", !!side && next.collapsed === true);
  if (side) wrap.dataset.edge = side;
  else delete wrap.dataset.edge;
}
function panel() {
  return `<div class="window-pad"><main class="panel" aria-label="Keeper 用量详情"><header class="panel-header"><div class="brand-row"><div class="logo">${icon("logo")}</div><div class="brand-title">Keeper <span class="brand-subtitle">用量面板</span></div><div class="spacer"></div><div id="connection" class="connection neutral"><i class="dot"></i>未连接</div><div class="header-actions"><button class="console-button" data-console="usage" title="在默认浏览器打开配置的 Keeper 地址">用量控制台</button><button class="console-button" data-console="cpa" id="cpa-console" disabled title="连接后获取 CPA 地址">CPA 控制台</button><button class="settings-button" data-action="settings">设置</button></div>${button("close-detail", "收起面板")}</div><div id="filters"></div><nav class="tabs" aria-label="指标分类">${availableTabs()
    .map(
      ([id, label, i]) =>
        `<button class="tab ${state.tab === id ? "active" : ""}" data-tab="${id}">${icon(i)}${label}</button>`,
    )
    .join(
      "",
    )}</nav></header><div id="connection-banner" hidden></div><section class="panel-content" id="content" aria-live="polite"></section><footer class="panel-footer"><span class="footer-lock">${icon("shield")}只读 · 北京时间</span><button data-action="refresh" id="updated-at">${icon("refresh")}等待采样</button></footer></main></div>`;
}
function renderFilters() {
  if (!$("#filters")) return;
  const more = !["today", "7d", "30d"].includes(state.range);
  $("#filters").innerHTML =
    `<div class="filter-row"><div class="segments">${["today", "7d", "30d"].map((r) => `<button class="${r === state.range ? "active" : ""}" data-pick="range" data-value="${r}">${ranges[r]}</button>`).join("")}</div>${picker(
      "range",
      more ? ranges[state.range] : "更多",
      [
        ["yesterday", "昨日"],
        ["month", "本月"],
        ["custom", "自定义"],
      ],
      state.range,
    )}${state.access?.role === "admin" ? picker("key", scopeLabel(), [["", "全部 Key"], ...state.keys.map((k) => [k.id, k.label])], state.key, "filters-key") : `<span class="viewer-scope" title="${e(scopeLabel())}">${icon("key")}<span>${e(scopeLabel())}</span></span>`}</div>${state.range === "custom" ? `<div class="custom-range"><input aria-label="开始日期" id="range-start" type="date" value="${e(state.start)}"><span class="muted">至</span><input aria-label="结束日期" id="range-end" type="date" value="${e(state.end)}"><button class="small-button" data-action="apply-range">应用</button></div>` : ""}`;
}
function scope() {
  return `<div class="section-top"><h2>${state.tab === "summary" ? "用量概览" : state.tab === "analysis" ? "成本拆分" : state.tab === "latency" ? "延迟诊断" : state.tab === "distribution" ? "用量分布" : "认证账户"}</h2><span class="scope-tag" title="${e(scopeLabel())}">${e(ranges[state.range])} · ${e(scopeLabel())}</span></div>`;
}
function empty(title, text, retry = true) {
  return `<div class="empty"><div class="empty-icon">${icon("link")}</div><h2>${e(title)}</h2><p>${e(text)}</p>${retry ? '<button class="small-button" data-action="refresh">重新读取</button>' : ""}</div>`;
}
function summary(data) {
  const usage = data.overview?.usage || {},
    sum = data.overview?.summary || {},
    a = data.activity || {};
  return (
    scope() +
    cards(
      [
        ["Token 总量", compact(usage.total_tokens), number(usage.total_tokens)],
        [
          "请求总数",
          number(usage.total_requests),
          `成功率 ${percent(usage.success_count, usage.total_requests)}`,
        ],
        [
          "缓存读取率",
          percent(sum.cache_read_tokens, sum.input_tokens),
          "缓存读 / 输入",
        ],
        ["总成本", cost(sum, "total_cost"), "API 等价估算", "cost"],
      ],
      4,
    ) +
    heading("Token 组成", "子项不重复计入总量") +
    rows([
      ["输入 · 含缓存", number(a.input_tokens)],
      ["输出 · 含推理", number(a.output_tokens)],
      [
        "缓存读取 / 写入",
        `${number(a.cache_read_tokens)} / ${number(a.cache_creation_tokens)}`,
      ],
      ["推理输出", number(a.reasoning_tokens)],
    ]) +
    `<div id="live-summary">${liveSummary()}</div>` +
    (state.access?.role === "api_key_viewer"
      ? note(
          "sk 只读权限 · 仅显示此 Key 用量；Keeper 未向此角色开放成本拆分、延迟诊断和认证账户指标。",
        )
      : state.key
        ? note(
            "Key owner 已同步到悬浮球。认证账户额度及错误无法按 Key 归属，仅在全部 Key 下展示。",
          )
        : "")
  );
}
function liveSummary() {
  const s = state.sample;
  if (!s) return note("下一次成功采样后显示当前范围的新增用量。");
  return `<div class="live-strip">${icon("shield")}<div><strong>${e(s.health.label)} <span>· ${s.health.basis === "key_requests" ? "此 Key 近 5 小时请求失败" : "近 5 小时认证失败"} ${number(s.health.failure)} 次</span></strong><p>${state.error ? "连接中断，等待重新采样" : s.delta.baseline ? "采样基线已建立，等待下一次更新" : `${Number(s.delta.seconds).toFixed(1)} 秒新增：输入 ${number(s.delta.input_tokens)} · 输出 ${number(s.delta.output_tokens)}`}</p></div></div>`;
}
function costs(data) {
  const b = data.cost_breakdown || {};
  return (
    scope() +
    cards([
      ["总成本", cost(b, "total_cost_usd"), "API 等价估算", "cost"],
      ["普通输入", cost(b, "uncached_input_cost_usd"), "不含缓存读 / 写"],
      ["缓存读取", cost(b, "cache_read_cost_usd")],
      ["缓存写入", cost(b, "cache_write_cost_usd")],
      ["输出", cost(b, "output_cost_usd"), "包含推理输出"],
    ]) +
    heading("模型效率") +
    table(
      ["模型", "请求", "成本 / 请求", "输出 / 请求", "缓存率"],
      (data.model_efficiency || []).map((m) => [
        m.model,
        number(m.requests),
        cost(m, "cost_per_request_usd"),
        number(m.output_tokens_per_request),
        percent(m.cache_read_tokens, m.input_tokens),
      ]),
    ) +
    note("成本按 Keeper 价格配置估算，不等于订阅实际扣费。缺少价格显示 —。")
  );
}
function latency(data) {
  if (data.supported === false)
    return empty(
      "此范围不支持延迟统计",
      "Keeper 延迟诊断限最近 30 天，请缩短日期范围。",
      false,
    );
  if (!data.total_points)
    return empty(
      "还没有延迟样本",
      "未上报延迟的记录不计作 0 ms，已有用量不受影响。",
      false,
    );
  return (
    scope() +
    cards([
      ["首 Token · P95", duration(data.p95_ttft_ms), "TTFT", "latency"],
      ["请求耗时 · P95", duration(data.p95_latency_ms), "端到端延迟"],
      ["最慢首 Token", duration(data.max_ttft_ms)],
      ["最长请求耗时", duration(data.max_latency_ms)],
    ]) +
    heading("样本信息") +
    rows([["有效样本", number(data.total_points)]])
  );
}
function distribution(data) {
  const fields = [
    ["model_composition", "按模型"],
    ["api_key_composition", "按 Key"],
    ["auth_files_composition", "按认证账户"],
    ["ai_provider_composition", "按提供商"],
  ];
  return (
    scope() +
    `<div class="subnav">${fields.map(([id, label]) => `<button data-pick="distribution" data-value="${id}" class="${state.distribution === id ? "active" : ""}">${label}</button>`).join("")}</div>` +
    table(
      ["名称", "Token", "占比", "请求", "成本"],
      [...(data[state.distribution] || [])]
        .sort((a, b) => b.total_tokens - a.total_tokens)
        .map((m) => [
          m.label,
          number(m.total_tokens),
          m.percent == null ? "—" : `${Number(m.percent).toFixed(1)}%`,
          number(m.requests),
          cost(m),
        ]),
    ) +
    note("模型、Key、账户和提供商是同一批用量的不同视角，不能相加。")
  );
}
function errorEvents(events) {
  return events.length
    ? events
        .map(
          (m) =>
            `<article class="error-event"><header><span class="num muted">${e(time(m.timestamp))}</span><span class="error-code">${e(m.status_code ?? "—")} · ${e(m.code || "未知错误")}</span></header><div class="error-model">${e(m.model || "—")}</div><p class="error-reason">${e(m.body_summary || "Keeper 未提供原因摘要")}</p><details><summary>诊断信息 ${icon("chevron")}</summary>${rows(
              [
                ["可重试", m.retryable ? "是" : "否"],
                ["账户重试时间", time(m.credential_retry_after)],
                ["模型重试时间", time(m.model_retry_after)],
              ],
            )}</details></article>`,
        )
        .join("")
    : '<div class="rows-card empty-inline">此范围暂无错误事件</div>';
}
function pagination(data) {
  return `<div class="pagination">${state.cursor ? '<button class="small-button" data-pick="cursor" data-value="">返回首屏</button>' : ""}${data.has_more && data.next_cursor ? `<button class="small-button" data-pick="cursor" data-value="${e(data.next_cursor)}">下一页 →</button>` : ""}</div>`;
}
function accountBody(account, data) {
  if (state.accountTab === "quota") {
    let html =
      note("账户累计概览与当前配额，不受上方日期 / Key 筛选影响。") +
      cards(
        [
          [
            "累计 Token",
            compact(account.total_tokens),
            `${number(account.total_tokens)} tokens`,
          ],
          ["累计请求", number(account.total_requests)],
          [
            "累计成功率",
            percent(
              account.success_count,
              (account.success_count || 0) + (account.failure_count || 0),
            ),
          ],
          [
            "累计缓存率",
            percent(account.cache_read_tokens, account.input_tokens),
          ],
        ],
        4,
      ) +
      heading("当前配额", "只读已有缓存");
    let any = false;
    for (const item of data.items || []) {
      const quota = item.quota?.quota || [];
      if (!quota.length) continue;
      any = true;
      html +=
        table(
          ["额度窗口", "剩余 / 已用", "重置时间", "周期 Token", "周期成本"],
          quota.map((m) => [
            m.label || m.key,
            m.remainingFraction != null
              ? `剩余 ${(m.remainingFraction * 100).toFixed(1)}%`
              : m.usedPercent != null
                ? `已用 ${Number(m.usedPercent).toFixed(1)}%`
                : m.remaining || "—",
            time(m.resetAt),
            number(m.window_usage_tokens),
            money(m.window_usage_cost),
          ]),
          ["name", "text", "time", "number", "number"],
        ) +
        note(
          `最近观测 ${time(item.refreshed_at)} · 套餐 ${item.quota?.subscription?.plan || "—"}`,
        );
    }
    return (
      html +
      (any ? "" : note("暂无已缓存的配额，本工具不会主动刷新上游额度。")) +
      heading("账户信息") +
      rows([
        [
          "提供商 / 类型",
          `${account.provider || "—"} / ${account.type || "—"}`,
        ],
        ["状态", account.disabled ? "已禁用" : "已启用"],
        ["最近使用", time(account.last_used_at)],
        [
          "近 5 小时成功 / 失败",
          `${number(account.credential_health?.total_success)} / ${number(account.credential_health?.total_failure)}`,
        ],
      ])
    );
  }
  if (state.accountTab === "quota-history") {
    const roles = `<div class="subnav">${[
      ["primary", "主额度"],
      ["secondary", "次额度"],
    ]
      .map(
        ([id, label]) =>
          `<button data-pick="role" data-value="${id}" class="${state.role === id ? "active" : ""}">${label}</button>`,
      )
      .join("")}</div>`;
    if (data.supported === false)
      return roles + note("此账户类型暂不支持 Keeper 额度历史。");
    const cycles = data.cycles || [];
    return (
      roles +
      note(
        "共享配额按真实周期统计，显示近 30 天已有观测，不按日期 / Key 拆分。",
      ) +
      heading("额度周期") +
      table(
        ["状态", "开始 / 重置", "初始 / 最近剩余", "请求", "Token", "成本"],
        cycles.map((c) => [
          c.status === "current" ? "当前" : "已结束",
          [time(c.window_started_at), time(c.reset_at)],
          `${c.first_remaining_percent ?? "—"}% / ${c.last_remaining_percent ?? "—"}%`,
          number(c.usage?.requests),
          number(c.usage?.total_tokens),
          cost(c.usage, "total_cost_usd"),
        ]),
        ["text", "time", "text", "number", "number", "number"],
      ) +
      heading("额度变化效率") +
      table(
        [
          "观察结束",
          "额度变化",
          "下降百分点",
          "区间 Token",
          "每百分点 Token",
          "区间成本",
          "每百分点成本",
        ],
        cycles
          .flatMap((c) => c.transitions || [])
          .map((t) => {
            const points =
              t.percentage_points ??
              Number(t.from_remaining_percent) - Number(t.to_remaining_percent);
            return [
              time(t.interval_ended_at),
              `${t.from_remaining_percent}% → ${t.to_remaining_percent}%`,
              number(points),
              number(t.usage?.total_tokens),
              number(t.tokens_per_point),
              cost(t.usage, "total_cost_usd"),
              cost(t, "cost_per_point", "cost_per_point_available"),
            ];
          }),
        ["time", "text", "number", "number", "number", "number", "number"],
      ) +
      note(
        "每百分点值 = 区间总量 ÷ 实际下降百分点；下降 1 个百分点时，两列数值相同。成本按同一区间动态计价后再除以下降百分点，缺少任一定价时显示 —。",
      )
    );
  }
  if (state.accountTab === "requests")
    return (
      note("按所选日期、Key 与当前账户筛选，不读取原始请求正文。") +
      heading("请求明细", "横向滚动查看完整指标") +
      requestTable(
        [
          "时间（北京）",
          "sk",
          "模型",
          "结果",
          "输入",
          "输出",
          "缓存读",
          "推理",
          "总 Token",
          "成本",
          "首 Token",
          "耗时",
        ],
        (data.events || []).map((m) => [
          time(m.timestamp),
          m.api_key || "—",
          m.model,
          m.failed ? "失败" : "成功",
          number(m.tokens?.input_tokens),
          number(m.tokens?.output_tokens),
          number(m.tokens?.cache_read_tokens),
          number(m.tokens?.reasoning_tokens),
          number(m.tokens?.total_tokens),
          cost(m),
          duration(m.ttft_ms),
          duration(m.latency_ms),
        ]),
        [
          "time",
          "name",
          "name",
          "text",
          "number",
          "number",
          "number",
          "number",
          "number",
          "number",
          "number",
          "number",
        ],
      ) +
      pagination(data)
    );
  return (
    note("按日期筛选本页，不支持 Key 归属。错误事件数不能与失败请求数相加。") +
    heading("本页范围内错误事件", number(data.total_count)) +
    errorEvents(data.events || []) +
    pagination(data) +
    note(data.scope_notice || "只显示本页条数，不代表日期范围总数。")
  );
}
function accounts(data) {
  const account = state.accounts.find((a) => String(a.id) === state.account);
  if (!account)
    return empty(
      "暂无认证账户",
      "这里只显示认证文件中的账户，不显示 API Key 身份。",
      false,
    );
  return (
    scope() +
    `<div class="account-selector"><div class="account-avatar">${icon("accounts")}</div>${picker(
      "account",
      account.displayName || account.name || account.identity,
      state.accounts.map((a) => [a.id, a.displayName || a.name || a.identity]),
      state.account,
    )}</div><div class="subnav">${accountTabs.map(([id, label]) => `<button data-pick="accountTab" data-value="${id}" class="${state.accountTab === id ? "active" : ""}">${label}</button>`).join("")}</div>` +
    accountBody(account, data) +
    `<div class="pagination">${state.accountPage > 1 ? '<button class="small-button" data-pick="accountPage" data-value="-1">上一页账户</button>' : ""}${state.accountPage < state.accountPages ? '<button class="small-button" data-pick="accountPage" data-value="1">下一页账户</button>' : ""}</div>`
  );
}
function query() {
  return {
    range: state.range,
    api_key_id: state.key,
    start: state.start,
    end: state.end,
    account_id: state.account,
    cursor: state.cursor,
    page: state.accountPage,
    window_role: state.role,
  };
}
function renderData() {
  if (!$("#content")) return;
  const renders = { summary, analysis: costs, latency, distribution, accounts };
  const content = $("#content");
  const requestView =
    state.tab === "accounts" && state.accountTab === "requests";
  content.classList.toggle("request-view", requestView);
  content.innerHTML = renders[state.tab](state.data);
  if (requestView) setupRequestTableScroll();
}

function setupRequestTableScroll() {
  const top = $(".request-table-scroll");
  const width = $(".request-table-scroll-width");
  const body = $(".request-table-wrap");
  if (!top || !width || !body) return;
  requestAnimationFrame(() => {
    width.style.width = `${body.scrollWidth}px`;
    top.scrollLeft = body.scrollLeft;
  });
  top.addEventListener("scroll", () => {
    body.scrollLeft = top.scrollLeft;
  });
  body.addEventListener("scroll", () => {
    top.scrollLeft = body.scrollLeft;
  });
}
async function load(background = false) {
  if (!$("#content") || !state.visible || !state.access) return;
  if (
    state.range === "custom" &&
    (!state.start || !state.end || state.start > state.end)
  ) {
    $("#content").innerHTML = empty(
      "日期范围无效",
      "请选择有效的开始和结束日期。",
      false,
    );
    return;
  }
  const gen = ++state.generation;
  state.loading = true;
  if (!background) {
    $("#content").classList.remove("request-view");
    $("#content").innerHTML =
      '<div class="skeleton"></div><div class="skeleton short"></div><div class="skeleton"></div>';
  }
  try {
    let data;
    if (state.tab === "accounts") {
      const result = await viewCall("accounts", query());
      if (gen !== state.generation) return;
      state.accounts = result.identities || [];
      state.accountPages = result.total_pages || 1;
      if (!state.accounts.some((a) => String(a.id) === state.account)) {
        state.account = String(state.accounts[0]?.id ?? "");
        state.cursor = "";
      }
      data = state.account ? await viewCall(state.accountTab, query()) : {};
    } else
      data = await viewCall(
        state.tab === "distribution" ? "analysis" : state.tab,
        query(),
      );
    if (gen !== state.generation) return;
    state.data = data;
    state.refreshedAt = Date.now();
    renderData();
  } catch (error) {
    if (gen === state.generation)
      $("#content").innerHTML = empty("暂时无法读取", String(error));
  } finally {
    if (gen === state.generation) state.loading = false;
  }
}
function updateSample() {
  const s = state.sample;
  const health = state.error ? "离线" : s?.health.label || "未连接";
  if ($("#widget")) {
    $("#widget").classList.toggle("offline", !!state.error);
    $("#today-total").textContent = compact(s?.today_tokens);
    $("#today-total").title = s ? number(s.today_tokens) + " tokens" : "";
    $("#health").className = `widget-health ${tone(health)}`;
    $("#health").setAttribute("aria-label", health);
    $("#health").title = health;
    $("#edge-health").className = `peek-health ${tone(health)}`;
    const edgeTotal = edgeCompact(s?.today_tokens);
    $("#edge-token-value").textContent = edgeTotal.value;
    $("#edge-token-unit").textContent = edgeTotal.unit;
    const delta = widgetDelta.update(s, !!state.error);
    $("#delta-input").textContent = compact(delta.input);
    $("#delta-output").textContent = compact(delta.output);
    $("#input-flow").classList.toggle(
      "flow-zero",
      delta.input === null || delta.input === 0,
    );
    $("#output-flow").classList.toggle(
      "flow-zero",
      delta.output === null || delta.output === 0,
    );
    $("#delta-input").title = number(delta.input);
    $("#delta-output").title = number(delta.output);
    // No native title tooltip over the compact widget; details have the full context.
    $("#widget").removeAttribute("title");
    $("#widget").setAttribute(
      "aria-label",
      `Keeper ${scopeLabel()}今日用量 ${s ? number(s.today_tokens) : "—"} Token，${health}，悬停或点击查看详情，拖动移动`,
    );
  }
  if ($("#connection")) {
    $("#connection").className =
      `connection ${state.error ? "amber" : s ? "green" : "neutral"}`;
    $("#connection").innerHTML =
      `<i class="dot"></i>${state.error ? "离线" : s ? "已连接" : "未连接"}`;
    const banner = $("#connection-banner");
    banner.hidden = !state.error;
    banner.className = "error-banner";
    banner.textContent = state.error
      ? `${state.error} · 已显示数据可能过期`
      : "";
    $("#updated-at").innerHTML =
      `${icon("refresh")}${s ? `${time(s.sampled_at).split(" ").at(-1)} 更新` : "刷新"}`;
    if ($("#live-summary")) $("#live-summary").innerHTML = liveSummary();
  }
}
async function openDetail() {
  if (!$("#content")) return;
  state.visible = true;
  if (!state.access) {
    try {
      await refreshAccess();
    } catch (error) {
      $("#content").innerHTML = empty("请连接 Keeper", String(error), false);
      return;
    }
  }
  state.range = "today";
  state.tab = "summary";
  state.cursor = "";
  renderFilters();
  document
    .querySelectorAll("[data-tab]")
    .forEach((b) => b.classList.toggle("active", b.dataset.tab === state.tab));
  await load();
}
function settings() {
  const s = state.settings;
  const sk = s.authMode === "api_key";
  const section = (id, title, hint, content) =>
    `<section class="setting-section" data-setting-section="${id}"><header><h2>${title}</h2><p>${hint}</p></header>${content}</section>`;
  const themeButtons = [
    ["light", "浅色"],
    ["dark", "深色"],
  ]
    .map(
      ([id, label]) =>
        `<button type="button" data-theme="${id}" class="${s.theme === id ? "active" : ""}">${label}</button>`,
    )
    .join("");
  const accent = /^#[0-9a-f]{6}$/i.test(s.accentColor || "")
    ? s.accentColor.toLowerCase()
    : "";
  const accentButtons = [
    ["#1756a9", "深海蓝"],
    ["#087f8c", "青绿"],
    ["#167144", "森林绿"],
    ["#9a5b00", "琥珀"],
    ["#b52a36", "砖红"],
    ["#8c3f78", "莓紫"],
  ]
    .map(
      ([color, label]) =>
        `<button type="button" class="accent-swatch ${accent === color ? "active" : ""}" data-accent="${color}" style="--swatch:${color}" title="${label}" aria-label="主题色：${label}"></button>`,
    )
    .join("");
  const connection = section(
    "connection",
    "连接参数",
    "Keeper 地址、登录凭据与网络通道",
    `<label class="field">Keeper 地址<input type="url" name="endpoint" required placeholder="https://keeper.example/usage" value="${e(s.endpoint || "")}" autocomplete="url"></label><p class="field-hint">填写完整页面地址；有 /usage 路径时请保留。</p><div class="preference-row"><label for="auth-mode">登录方式</label><select id="auth-mode" name="authMode"><option value="admin" ${sk ? "" : "selected"}>管理员密码</option><option value="api_key" ${sk ? "selected" : ""}>API Key（sk）</option></select></div><label class="field"><span id="credential-label">${sk ? "CPA API Key（sk）" : "管理员登录密码"}</span><input type="password" name="password" placeholder="${s.hasPassword ? "已保存凭据，留空继续使用" : sk ? "sk-…" : "Keeper 管理员密码"}" autocomplete="current-password" spellcheck="false"></label><p class="field-hint" id="auth-hint">${sk ? "仅可查看此 Key 的用量，不开放管理员指标。" : "可查看全部用量，或按 Key owner 筛选。"}</p><label class="check-row"><input type="checkbox" name="rememberPassword" ${s.rememberPassword ? "checked" : ""}>记住登录凭据 · Windows 用户加密</label>${s.hasPassword ? '<label class="check-row" id="clear-credential"><input type="checkbox" name="clearPassword">清除已保存凭据（无密码 Keeper）</label>' : ""}<label class="check-row"><input type="checkbox" name="allowPrivateHttp" ${s.allowPrivateHttp ? "checked" : ""}>允许受保护专网内的 HTTP 连接</label><label class="check-row tls-warning"><input type="checkbox" name="allowInvalidCertificates" ${s.allowInvalidCertificates ? "checked" : ""}><span><strong>忽略 HTTPS 证书验证</strong><small>仅限受信任内网；会接受伪造或过期证书，建议优先安装内网代理 CA。</small></span></label><details class="proxy-settings" ${s.proxyUrl ? "open" : ""}><summary>代理设置 <span class="muted">· 可选</span>${icon("chevron")}</summary><label class="field">HTTP / SOCKS5 代理<input name="proxyUrl" type="text" placeholder="socks5://127.0.0.1:1080" value="${e(s.proxyUrl || "")}" autocomplete="off"></label><p class="field-hint">留空直连。支持 http://、socks5://、socks5h://；认证格式为 scheme://用户:密码@主机:端口，特殊字符需 URL 编码。代理地址加密保存。</p></details>`,
  );
  const appearance = section(
    "appearance",
    "外观与样式",
    "主题、强调色与悬浮窗字体",
    `<div class="preference-row"><label>明暗主题</label><div class="segments">${themeButtons}</div></div><div class="accent-setting"><div class="preference-row"><label for="accent-color-picker">主题色</label><button type="button" class="accent-default ${accent ? "" : "active"}" data-accent="">跟随主题</button></div><input type="hidden" name="accentColor" value="${e(accent)}"><div class="accent-palette">${accentButtons}<label class="accent-picker" title="打开系统调色盘"><input id="accent-color-picker" type="color" value="${e(accent || (s.theme === "dark" ? "#8bbbff" : "#1756a9"))}" aria-label="自定义主题色"><span>自定义</span></label></div></div><label class="field">悬浮窗字体<input name="widgetFont" type="text" list="font-options" placeholder="HarmonyOS Sans SC" value="${e(s.widgetFont || "HarmonyOS Sans SC")}"></label><datalist id="font-options"><option value="HarmonyOS Sans SC"><option value="Microsoft YaHei UI"><option value="Microsoft YaHei"><option value="Segoe UI"><option value="Noto Sans SC"></datalist><p class="field-hint">未安装时自动回退：鸿蒙黑体 → 微软雅黑 → 系统无衬线字体。</p>`,
  );
  const behavior = section(
    "behavior",
    "悬浮窗行为",
    "刷新频率、数据展示与自动隐藏",
    `<div class="preference-row"><label for="poll-seconds">刷新间隔 <span class="muted">/ 秒</span></label><input id="poll-seconds" name="pollSeconds" type="number" min="1" max="60" value="${s.pollSeconds}" required></div><div class="preference-row"><label for="display-hold-seconds">非零数据保留 <span class="muted">/ 秒</span></label><input id="display-hold-seconds" name="displayHoldSeconds" type="number" min="0" max="300" value="${s.displayHoldSeconds ?? 16}" required></div><p class="field-hint setting-hint">连续收到零用量达到此时长后归零；设为 0 即立即归零。</p><label class="check-row setting-toggle"><input type="checkbox" name="edgeAutoCollapse" ${s.edgeAutoCollapse !== false ? "checked" : ""}><span><strong>贴近屏幕边缘自动收起</strong><small>拖到屏幕左右外沿时折叠，移入鼠标后展开。</small></span></label><label class="check-row setting-toggle"><input type="checkbox" name="fullscreenAutoHide" ${s.fullscreenAutoHide !== false ? "checked" : ""}><span><strong>全屏时自动隐藏悬浮窗</strong><small>游戏、视频或无边框全屏结束后自动恢复展示。</small></span></label><label class="check-row setting-toggle"><input type="checkbox" name="autoStart" ${s.autoStart ? "checked" : ""}><span><strong>登录 Windows 后启动</strong><small>随当前用户会话自动启动用量面板。</small></span></label>`,
  );
  return `<div class="window-pad"><main class="panel settings"><header class="panel-header"><div class="brand-row"><div class="logo">${icon("logo")}</div><div class="brand-title">Keeper <span class="brand-subtitle">用量面板</span></div><span class="spacer"></span><span class="eyebrow">设置</span>${button("close-settings", "关闭设置")}</div></header><form id="settings-form" class="settings-form"><div class="settings-body">${connection}${appearance}${behavior}</div><footer class="settings-actions"><div class="settings-error" id="settings-error" role="alert"></div><button type="submit" class="connect-button" id="save-settings">保存并连接 ${icon("arrow")}</button><div class="registry-note">${icon("shield")}配置保存在当前用户注册表 · 无需远程服务</div></footer></form><section class="connection-error-dialog" id="connection-error-dialog" role="dialog" aria-modal="true" aria-labelledby="connection-error-title" aria-describedby="connection-error-summary" hidden><div class="connection-error-card"><div class="error-dialog-kicker">CONNECTION / ERRLOG</div><h2 id="connection-error-title">Keeper 连接失败</h2><p id="connection-error-summary"></p><pre id="connection-errlog" tabindex="0"></pre><p class="error-dialog-hint">日志已隐藏登录凭据与代理认证信息，可直接复制用于排查。</p><div class="error-dialog-actions"><button type="button" data-error-action="close">返回设置</button><button type="button" class="copy-errlog" data-error-action="copy">复制 ERRLOG</button></div></div></section></main></div>`;
}

root.innerHTML = preview
  ? `<div class="preview-stage ${search.has("standalone") ? "standalone" : ""} ${windowName === "settings" ? "settings-preview" : windowName === "widget" ? "widget-only" : ""}">${windowName === "settings" ? "" : `<div class="preview-widget">${widget()}</div>`}<div class="preview-panel">${windowName === "settings" ? "" : panel()}</div><div class="preview-label">KEEPER / 0.4　·　界面预览，示例数据</div></div>`
  : windowName === "widget"
    ? widget()
    : windowName === "settings"
      ? ""
      : panel();

document.addEventListener("click", async (event) => {
  const b = event.target.closest("button");
  if (!b) return;
  if (b.dataset.errorAction === "close") {
    closeConnectionError();
    return;
  }
  if (b.dataset.errorAction === "copy") {
    await copyConnectionErrlog(b);
    return;
  }
  if (b.dataset.console) {
    b.disabled = true;
    try {
      await api.call("open_console", { target: b.dataset.console });
      $("#console-error")?.remove();
    } catch (error) {
      $("#console-error")?.remove();
      $("#content").insertAdjacentHTML(
        "afterbegin",
        `<div class="note" id="console-error" role="alert">${e(String(error))}</div>`,
      );
    } finally {
      b.disabled =
        b.dataset.console === "cpa" && state.access?.role !== "admin";
    }
    return;
  }
  if ("accent" in b.dataset) {
    const accent = b.dataset.accent;
    state.settings.accentColor = accent;
    const hidden = $('[name="accentColor"]');
    const picker = $("#accent-color-picker");
    if (hidden) hidden.value = accent;
    if (picker && accent) picker.value = accent;
    document
      .querySelectorAll("[data-accent]")
      .forEach((el) => el.classList.toggle("active", el === b));
    applyAppearance();
    return;
  }
  if (b.dataset.action) {
    const name = b.dataset.action;
    if (name === "refresh") {
      state.cursor = "";
      await load();
    } else if (name === "apply-range") {
      state.start = $("#range-start").value;
      state.end = $("#range-end").value;
      state.cursor = "";
      await load();
    } else if (name === "settings" && preview) {
      $(".preview-widget")?.remove();
      $(".preview-stage").classList.add("settings-preview");
      $(".preview-panel").innerHTML = settings();
    } else await action(name);
    return;
  }
  if (b.dataset.tab) {
    state.tab = b.dataset.tab;
    state.cursor = "";
    document
      .querySelectorAll("[data-tab]")
      .forEach((el) => el.classList.toggle("active", el === b));
    await load();
    return;
  }
  if (b.dataset.theme) {
    state.settings.theme = b.dataset.theme;
    applyAppearance();
    document
      .querySelectorAll("[data-theme]")
      .forEach((el) => el.classList.toggle("active", el === b));
    return;
  }
  if (b.dataset.pick) {
    const { pick, value } = b.dataset;
    b.closest("details")?.removeAttribute("open");
    if (pick === "key") {
      if (state.switchingScope) return;
      state.switchingScope = true;
      try {
        applyAccess(await api.call("set_scope", { apiKeyId: value }));
      } catch (error) {
        $("#content").innerHTML = empty("无法切换 Key owner", String(error));
      } finally {
        state.switchingScope = false;
      }
      return;
    }
    state.cursor = pick === "cursor" ? value : "";
    if (pick === "accountPage") {
      state.accountPage += Number(value);
      state.account = "";
    } else state[pick] = value;
    if (pick === "key" || pick === "range") {
      renderFilters();
      if (pick === "range" && value === "custom") return;
    }
    if (pick === "distribution") {
      renderData();
      return;
    }
    await load();
  }
});
document.addEventListener("click", (event) => {
  document.querySelectorAll("details[open]").forEach((d) => {
    if (!d.contains(event.target)) d.removeAttribute("open");
  });
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    if (!$("#connection-error-dialog")?.hidden) {
      closeConnectionError();
      return;
    }
    const open = $("details[open]");
    if (open) {
      open.removeAttribute("open");
      open.querySelector("summary").focus();
    } else
      action(windowName === "settings" ? "close-settings" : "close-detail");
  }
  if (event.key === "Enter" && event.target.id === "widget") action("detail");
});
document.addEventListener("change", (event) => {
  if (event.target.id !== "auth-mode") return;
  const sk = event.target.value === "api_key";
  const field = $("[name=password]");
  field.value = "";
  field.placeholder =
    state.settings.hasPassword &&
    event.target.value === (state.settings.authMode || "admin")
      ? "已保存凭据，留空继续使用"
      : sk
        ? "sk-…"
        : "Keeper 管理员密码";
  $("#credential-label").textContent = sk
    ? "CPA API Key（sk）"
    : "管理员登录密码";
  $("#auth-hint").textContent = sk
    ? "仅可查看此 Key 的用量，不开放管理员指标。"
    : "可查看全部用量，或按 Key owner 筛选。";
  if ($("#clear-credential")) $("#clear-credential").hidden = sk;
});
document.addEventListener("input", (event) => {
  if (event.target.id !== "accent-color-picker") return;
  state.settings.accentColor = event.target.value.toLowerCase();
  $('[name="accentColor"]').value = state.settings.accentColor;
  document
    .querySelectorAll("[data-accent]")
    .forEach((el) => el.classList.remove("active"));
  applyAppearance();
});
document.addEventListener("submit", async (event) => {
  if (event.target.id !== "settings-form") return;
  event.preventDefault();
  const form = new FormData(event.target);
  const save = $("#save-settings");
  save.disabled = true;
  save.textContent = "正在验证连接…";
  $("#settings-error").textContent = "";
  try {
    await api.call("save_settings", {
      value: {
        ...state.settings,
        endpoint: form.get("endpoint"),
        password: form.get("password"),
        authMode: form.get("authMode"),
        proxyUrl: form.get("proxyUrl"),
        widgetFont: form.get("widgetFont"),
        accentColor: form.get("accentColor"),
        pollSeconds: Number(form.get("pollSeconds")),
        displayHoldSeconds: Number(form.get("displayHoldSeconds")),
        rememberPassword: form.has("rememberPassword"),
        allowPrivateHttp: form.has("allowPrivateHttp"),
        allowInvalidCertificates: form.has("allowInvalidCertificates"),
        edgeAutoCollapse: form.has("edgeAutoCollapse"),
        fullscreenAutoHide: form.has("fullscreenAutoHide"),
        autoStart: form.has("autoStart"),
      },
      clearPassword: form.has("clearPassword"),
    });
    $("#settings-error").textContent = preview
      ? "预览模式：未写入注册表。"
      : "";
  } catch (error) {
    showConnectionError(error);
  } finally {
    save.disabled = false;
    save.innerHTML = `保存并连接 ${icon("arrow")}`;
  }
});
if ($("#widget")) {
  let down = null,
    dragging = false;
  $("#widget").addEventListener("pointerdown", (event) => {
    if (event.button === 0 && !event.target.closest("button")) {
      down = { x: event.screenX, y: event.screenY };
      dragging = false;
    }
  });
  $("#widget").addEventListener("pointermove", (event) => {
    if (
      down &&
      !dragging &&
      event.buttons === 1 &&
      Math.hypot(event.screenX - down.x, event.screenY - down.y) > 4
    ) {
      dragging = true;
      down = null;
      $("#widget").classList.add("dragging");
      action("drag").then((ok) => {
        if (!ok) cancelDrag();
      });
    }
  });
  $("#widget").addEventListener("pointerup", () => {
    if (down && !dragging) action("detail");
    down = null;
    $("#widget").classList.remove("dragging");
  });
  const cancelDrag = () => {
    down = null;
    dragging = false;
    $("#widget").classList.remove("dragging");
  };
  $("#widget").addEventListener("pointercancel", cancelDrag);
  window.addEventListener("blur", cancelDrag);
  api.on("drag-finished", cancelDrag);
  $("#widget").addEventListener("contextmenu", (event) => {
    event.preventDefault();
    if ($("#widget-menu")) {
      $("#widget-menu").remove();
      return;
    }
    $("#widget").insertAdjacentHTML(
      "beforeend",
      `<div class="widget-menu" id="widget-menu"><button data-action="settings">设置</button><button data-action="hide">隐藏</button><button data-action="quit">退出</button></div>`,
    );
  });
  $("#widget").addEventListener("pointerleave", () => {
    $("#widget-menu")?.remove();
  });
}
function applyAccess(access) {
  if (access.scope.revision < state.revision) return;
  const changed =
    !state.access ||
    access.scope.revision !== state.revision ||
    access.role !== state.access.role;
  state.access = access;
  state.revision = access.scope.revision;
  state.key = access.scope.api_key_id;
  if (changed) {
    state.generation++;
    state.loading = false;
    state.sample = null;
    state.error = "";
    state.data = null;
    state.cursor = "";
    state.accounts = [];
    state.account = "";
    widgetDelta.clear();
    if (!availableTabs().some(([id]) => id === state.tab))
      state.tab = "summary";
    updateSample();
  }
  const nav = $(".tabs");
  if (nav)
    nav.innerHTML = availableTabs()
      .map(
        ([id, label, i]) =>
          `<button class="tab ${state.tab === id ? "active" : ""}" data-tab="${id}">${icon(i)}${label}</button>`,
      )
      .join("");
  const cpa = $("#cpa-console");
  if (cpa) {
    cpa.disabled = access.role !== "admin";
    cpa.title = cpa.disabled
      ? "Keeper 仅向管理员提供 CPA 控制台地址"
      : "从 Keeper 获取地址，在默认浏览器打开 CPA 控制台";
  }
  renderFilters();
  if (changed && $("#content")) {
    $("#content").innerHTML = "";
    load();
  }
}
async function refreshAccess() {
  const epoch = state.epoch;
  const access = await api.call("get_access");
  if (epoch !== state.epoch) return;
  applyAccess(access);
  if ($("#filters") && access.role === "admin") {
    const revision = state.revision;
    const keys = await viewCall("keys", { range: "today" }).catch(() => ({}));
    if (epoch !== state.epoch || revision !== state.revision) return;
    state.keys = keys.options || [];
    renderFilters();
  }
}
function acceptSample(s) {
  if (s && s.revision !== undefined && s.revision !== state.revision) return;
  state.sample = s;
  state.error = "";
  updateSample();
}
await api.on("configured", async ({ settings: s, revision }) => {
  state.revision = revision;
  state.epoch++;
  state.generation++;
  state.loading = false;
  state.settings = s;
  state.access = null;
  state.sample = null;
  state.error = "";
  state.keys = [];
  applySettings();
  updateSample();
  if (windowName !== "settings") {
    if ($("#content")) $("#content").innerHTML = "";
    if ($(".tabs")) $(".tabs").innerHTML = "";
    if ($("#cpa-console")) $("#cpa-console").disabled = true;
    try {
      await refreshAccess();
    } catch (error) {
      state.error = String(error);
      updateSample();
    }
  }
});
await api.on("settings-open", async () => {
  if (windowName === "settings") {
    state.settings = await api.call("get_settings");
    applySettings();
    root.innerHTML = settings();
  }
});
if (windowName !== "settings") {
  await api.on("scope-changed", applyAccess);
  await api.on("sample", acceptSample);
  await api.on("connection-error", (error) => {
    if (typeof error === "object" && error.revision !== state.revision) return;
    state.error = String(error.message || error);
    updateSample();
  });
  await api.on("detail-open", openDetail);
  await api.on("detail-close", () => {
    state.visible = false;
    state.generation++;
    state.loading = false;
  });
}
if ($("#widget")) {
  await api.on("widget-edge", applyWidgetEdge);
  try {
    applyWidgetEdge(await api.call("widget_edge_state"));
  } catch (error) {
    console.error(error);
  }
}
try {
  state.settings = await api.call("get_settings");
  applySettings();
  if (windowName === "settings") {
    if (preview) $(".preview-panel").innerHTML = settings();
    else root.innerHTML = settings();
  } else {
    await refreshAccess();
    acceptSample(await api.call("last_sample"));
  }
} catch (error) {
  state.error = String(error);
  updateSample();
  if ($("#content"))
    $("#content").innerHTML = empty("暂时无法读取", String(error));
}

if (windowName === "widget" || (preview && windowName !== "settings")) {
  const poll = async () => {
    const started = performance.now();
    if (state.settings.endpoint && !document.hidden) {
      const epoch = state.epoch,
        revision = state.revision;
      try {
        const next = await api.call("sample");
        if (epoch === state.epoch && revision === state.revision)
          acceptSample(next);
      } catch (error) {
        if (epoch === state.epoch && revision === state.revision) {
          state.error = String(error);
          updateSample();
        }
      }
    }
    setTimeout(
      poll,
      Math.max(
        250,
        (state.settings.pollSeconds || 2) * 1000 -
          (performance.now() - started),
      ),
    );
  };
  poll();
}
if ($("#content"))
  setInterval(() => {
    if (
      state.visible &&
      !state.loading &&
      !$("details[open]") &&
      !document.hidden
    ) {
      const now = Date.now(),
        interval =
          state.tab === "summary"
            ? 10000
            : state.tab === "accounts"
              ? 60000
              : 30000;
      if (now - (state.refreshedAt || 0) >= interval) {
        state.refreshedAt = now;
        load(true);
      }
    }
  }, 2000);
