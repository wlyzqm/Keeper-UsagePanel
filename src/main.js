import "./style.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { icon } from "./icons.js";
import {
  escape as e,
  number,
  compact,
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
const state = {
  settings: { pollSeconds: 2, theme: "light" },
  sample: null,
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
  document.documentElement.style.setProperty(
    "--widget-font",
    widgetFontStack(state.settings.widgetFont),
  );
};
const action = async (name) => {
  try {
    await api.call("window_action", { action: name });
  } catch (error) {
    console.error(error);
  }
};
const button = (name, title) =>
  `<button class="icon-button" data-action="${name}" title="${title}" aria-label="${title}">${icon(name === "close-detail" || name === "close-settings" ? "close" : name)}</button>`;
const note = (text) =>
  `<div class="note">${icon("info")}<p>${e(text)}</p></div>`;
const rows = (items) =>
  `<div class="rows-card">${items.map(([label, value]) => `<div class="metric-row"><span>${e(label)}</span><strong class="num">${e(value)}</strong></div>`).join("")}</div>`;
const cards = (items, three = false) =>
  `<div class="metric-grid ${three ? "three" : ""}">${items.map(([label, value, hint, mark]) => `<div class="metric-card"><div class="metric-label">${mark ? icon(mark) : ""}${e(label)}</div><div class="metric-value num ${String(value).length > 13 ? "small" : ""}">${e(value)}</div>${hint ? `<p>${e(hint)}</p>` : ""}</div>`).join("")}</div>`;
const heading = (text, right = "") =>
  `<div class="group-title"><span>${e(text)}</span><span class="muted">${e(right)}</span></div>`;
const table = (headers, items) =>
  items.length
    ? `<div class="table-wrap"><table class="data-table"><thead><tr>${headers.map((h) => `<th>${e(h)}</th>`).join("")}</tr></thead><tbody>${items.map((row) => `<tr>${row.map((v) => `<td>${e(v ?? "—")}</td>`).join("")}</tr>`).join("")}</tbody></table></div>`
    : `<div class="rows-card empty-inline">此范围暂无记录</div>`;
const picker = (name, label, options, selected, extra = "") =>
  `<details class="picker ${extra}"><summary aria-label="${name === "key" ? "按 Key 筛选" : name === "range" ? "更多日期范围" : name === "account" ? "选择认证账户" : "切换分布维度"}">${name === "key" ? icon("key") : ""}<span class="picker-label">${e(label)}</span>${icon("chevron")}</summary><div class="picker-menu">${options.map(([value, text]) => `<button data-pick="${name}" data-value="${e(value)}" class="${String(value) === String(selected) ? "selected" : ""}" title="${e(text)}">${e(text)}</button>`).join("")}</div></details>`;

function widget() {
  return `<div class="widget-wrap"><div class="widget" id="widget" role="button" tabindex="0" aria-label="Keeper 全局用量，悬停查看详情，拖动移动"><div class="orb neutral"><div class="orb-label">今日 TOKEN</div><div class="orb-number num" id="today-total">—</div><div class="orb-health" id="health"><i class="dot"></i>未连接</div></div><div class="widget-flows"><div class="flow-row green">${icon("input")}<span>输入</span><strong class="num" id="delta-input">—</strong></div><div class="flow-row">${icon("output")}<span>输出</span><strong class="num" id="delta-output">—</strong></div><div class="widget-interval" id="interval-label">等待首次采样</div></div></div></div>`;
}
function panel() {
  return `<div class="window-pad"><main class="panel" aria-label="Keeper 用量详情"><header class="panel-header"><div class="brand-row"><div class="logo">${icon("logo")}</div><div><div class="brand-title">Keeper<span style="font-weight:400;color:var(--muted);font-size:12px;letter-spacing:0;margin-left:8px">用量面板</span></div><div class="brand-caption">A LITTLE WINDOW INTO YOUR USAGE</div></div><div class="spacer"></div><div id="connection" class="connection neutral"><i class="dot"></i>未连接</div>${button("settings", "连接设置")}${button("close-detail", "收起面板")}</div><div id="filters"></div><nav class="tabs" aria-label="指标分类">${tabs.map(([id, label, i]) => `<button class="tab ${state.tab === id ? "active" : ""}" data-tab="${id}">${icon(i)}${label}</button>`).join("")}</nav></header><div id="connection-banner" hidden></div><section class="panel-content" id="content" aria-live="polite"></section><footer class="panel-footer"><span class="footer-lock">${icon("shield")}只读连接 · 北京时间</span><button data-action="refresh" id="updated-at">${icon("refresh")}等待采样</button></footer></main></div>`;
}
function renderFilters() {
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
    )}${picker("key", state.keys.find((k) => String(k.id) === state.key)?.label || "全部 Key", [["", "全部 Key"], ...state.keys.map((k) => [k.id, k.label])], state.key, "filters-key")}</div>${state.range === "custom" ? `<div class="custom-range"><input aria-label="开始日期" id="range-start" type="date" value="${e(state.start)}"><span class="muted">至</span><input aria-label="结束日期" id="range-end" type="date" value="${e(state.end)}"><button class="small-button" data-action="apply-range">应用</button></div>` : ""}`;
}
function scope() {
  return `<div class="section-top"><h2>${state.tab === "summary" ? "用量概览" : state.tab === "analysis" ? "成本拆分" : state.tab === "latency" ? "延迟诊断" : state.tab === "distribution" ? "用量分布" : "认证账户"}</h2><span class="scope-tag" title="${e(state.keys.find((k) => String(k.id) === state.key)?.label || "全部 Key")}">${e(ranges[state.range])} · ${e(state.keys.find((k) => String(k.id) === state.key)?.label || "全部 Key")}</span></div>`;
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
    `<div class="metric-hero"><div><div class="metric-label">Token 总用量</div><div class="hero-value num" title="${number(usage.total_tokens)}">${compact(usage.total_tokens)}</div><div class="metric-foot">${number(usage.total_tokens)} tokens</div></div><div><div class="metric-label">Keeper 请求总数</div><div class="hero-value secondary-value num">${number(usage.total_requests)}</div><div class="metric-foot">成功率 ${percent(usage.success_count, usage.total_requests)}</div></div></div>` +
    cards(
      [
        [
          "缓存读取率",
          percent(sum.cache_read_tokens, sum.input_tokens),
          "缓存读 / 输入",
        ],
        ["总成本", cost(sum, "total_cost"), "API 等价估算", "cost"],
      ],
      false,
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
    `<div id="live-summary">${liveSummary()}</div>`
  );
}
function liveSummary() {
  const s = state.sample;
  if (!s) return note("下一次成功采样后显示全局新增用量。");
  return `<div class="live-strip">${icon("shield")}<div><strong>全局${e(s.health.label)} <span style="font-weight:400;opacity:.7">· 近 5 小时失败 ${number(s.health.failure)} 次</span></strong><p>${state.error ? "连接中断，等待重新采样" : s.delta.baseline ? "采样基线已建立，等待下一次更新" : `${Number(s.delta.seconds).toFixed(1)} 秒新增：输入 ${number(s.delta.input_tokens)} · 输出 ${number(s.delta.output_tokens)}`}</p></div></div>`;
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
    rows([
      ["有效样本", number(data.total_points)],
      ["统计口径", "直接采用 Keeper 汇总值"],
    ]) +
    note(
      "仅包含有效上报的延迟。导入的零延迟不代表瞬间完成；不从散点反推全量分位数。",
    )
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
      cards([
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
      ]) +
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
      ]) +
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
        ) +
        note(
          `最近观测 ${time(item.refreshed_at)} · 套餐 ${item.quota?.subscription?.plan || "—"}`,
        );
    }
    return (
      html + (any ? "" : note("暂无已缓存的配额，本工具不会主动刷新上游额度。"))
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
          `${time(c.window_started_at)} / ${time(c.reset_at)}`,
          `${c.first_remaining_percent ?? "—"}% / ${c.last_remaining_percent ?? "—"}%`,
          number(c.usage?.requests),
          number(c.usage?.total_tokens),
          cost(c.usage, "total_cost_usd"),
        ]),
      ) +
      heading("额度变化效率") +
      table(
        ["观察结束", "额度变化", "Token", "每百分点 Token", "每百分点成本"],
        cycles
          .flatMap((c) => c.transitions || [])
          .map((t) => [
            time(t.interval_ended_at),
            `${t.from_remaining_percent}% → ${t.to_remaining_percent}%`,
            number(t.usage?.total_tokens),
            number(t.tokens_per_point),
            cost(t, "cost_per_point", "cost_per_point_available"),
          ]),
      ) +
      note("未观测到的日期不补零，百分点不等同于 Token。")
    );
  }
  if (state.accountTab === "requests")
    return (
      note("按所选日期、Key 与当前账户筛选，不读取原始请求正文。") +
      heading("请求明细") +
      table(
        [
          "时间（北京）",
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
  $("#content").innerHTML = renders[state.tab](state.data);
}
async function load(background = false) {
  if (!$("#content") || !state.visible) return;
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
  if (!background)
    $("#content").innerHTML =
      '<div class="skeleton"></div><div class="skeleton short"></div><div class="skeleton"></div>';
  try {
    let data;
    if (state.tab === "accounts") {
      const result = await api.call("get_view", {
        view: "accounts",
        query: query(),
      });
      if (gen !== state.generation) return;
      state.accounts = result.identities || [];
      state.accountPages = result.total_pages || 1;
      if (!state.accounts.some((a) => String(a.id) === state.account)) {
        state.account = String(state.accounts[0]?.id ?? "");
        state.cursor = "";
      }
      data = state.account
        ? await api.call("get_view", { view: state.accountTab, query: query() })
        : {};
    } else
      data = await api.call("get_view", {
        view: state.tab === "distribution" ? "analysis" : state.tab,
        query: query(),
      });
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
    $(".orb").className = `orb ${tone(health)}`;
    $("#today-total").textContent = compact(s?.today_tokens);
    $("#today-total").title = s ? number(s.today_tokens) + " tokens" : "";
    $("#health").innerHTML = `<i class="dot"></i>${e(health)}`;
    $("#delta-input").textContent =
      s && !state.error && !s.delta.baseline
        ? compact(s.delta.input_tokens)
        : "—";
    $("#delta-output").textContent =
      s && !state.error && !s.delta.baseline
        ? compact(s.delta.output_tokens)
        : "—";
    $("#interval-label").textContent = state.error
      ? "等待重新连接"
      : !s
        ? "等待首次采样"
        : s.delta.baseline
          ? "已建立采样基线"
          : `${s.delta.seconds.toFixed(1)}s 内新增`;
    $("#widget").title =
      state.error ||
      "全局统计 · 北京时间\n悬停查看详情，拖动移动，右键打开菜单";
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
  state.visible = true;
  state.range = "today";
  state.key = "";
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
  return `<div class="window-pad"><main class="panel settings"><header class="panel-header"><div class="brand-row"><div class="logo">${icon("logo")}</div><div class="brand-title">Keeper</div><span class="spacer"></span><span class="eyebrow">连接设置</span>${button("close-settings", "关闭设置")}</div></header><form id="settings-form" class="settings-form"><div class="settings-body"><div class="settings-intro"><h1>让用量，触手可及。</h1><p>连接已有的 Keeper，桌面上的一小处，<br>就能看见每一次用量变化。</p></div><label class="field">Keeper 地址<input type="url" name="endpoint" required placeholder="https://keeper.example/usage" value="${e(s.endpoint || "")}" autocomplete="url"></label><p class="field-hint">填写完整页面地址；有 /usage 路径时请保留。</p><label class="field">登录密码<input type="password" name="password" placeholder="${s.hasPassword ? "已保存密码，留空继续使用" : "Keeper 登录密码，不是 API Key"}" autocomplete="current-password"></label><label class="check-row"><input type="checkbox" name="rememberPassword" ${s.rememberPassword ? "checked" : ""}>记住密码 · Windows 用户加密</label>${s.hasPassword ? '<label class="check-row"><input type="checkbox" name="clearPassword">清除已保存密码（适用于无密码 Keeper）</label>' : ""}<label class="check-row"><input type="checkbox" name="allowPrivateHttp" ${s.allowPrivateHttp ? "checked" : ""}>允许受保护专网内的 HTTP 连接</label><details class="proxy-settings" ${s.proxyUrl ? "open" : ""}><summary>代理设置 <span class="muted">· 可选</span>${icon("chevron")}</summary><label class="field">HTTP / SOCKS5 代理<input name="proxyUrl" type="text" placeholder="socks5://127.0.0.1:1080" value="${e(s.proxyUrl || "")}" autocomplete="off"></label><p class="field-hint">留空直连。支持 http://、socks5://、socks5h://；认证格式为 scheme://用户:密码@主机:端口，特殊字符需 URL 编码。代理地址加密保存。</p></details><hr class="setting-divider"><div class="preference-row"><label for="poll-seconds">刷新间隔 <span class="muted">/ 秒</span></label><input id="poll-seconds" name="pollSeconds" type="number" min="1" max="60" value="${s.pollSeconds}" required></div><div class="preference-row"><label>外观</label><div class="segments">${[
    ["light", "浅色"],
    ["dark", "深色"],
  ]
    .map(
      ([id, label]) =>
        `<button type="button" data-theme="${id}" class="${s.theme === id ? "active" : ""}">${label}</button>`,
    )
    .join(
      "",
    )}</div></div><label class="field">悬浮球字体<input name="widgetFont" type="text" list="font-options" placeholder="HarmonyOS Sans SC" value="${e(s.widgetFont || "HarmonyOS Sans SC")}"></label><datalist id="font-options"><option value="HarmonyOS Sans SC"><option value="Microsoft YaHei UI"><option value="Microsoft YaHei"><option value="Segoe UI"><option value="Noto Sans SC"></datalist><p class="field-hint">未安装时自动回退：鸿蒙黑体 → 微软雅黑 → 系统无衬线字体。</p><label class="check-row"><input type="checkbox" name="autoStart" ${s.autoStart ? "checked" : ""}>登录 Windows 后启动</label></div><footer class="settings-actions"><div class="settings-error" id="settings-error" role="alert"></div><button type="submit" class="connect-button" id="save-settings">保存并连接 ${icon("arrow")}</button><div class="registry-note">${icon("shield")}配置保存在当前用户注册表 · 无需远程服务</div></footer></form></main></div>`;
}

root.innerHTML = preview
  ? `<div class="preview-stage ${windowName === "settings" ? "settings-preview" : windowName === "widget" ? "widget-only" : ""}">${windowName === "settings" ? "" : `<div class="preview-widget">${widget()}</div>`}<div class="preview-panel">${windowName === "settings" ? "" : panel()}</div><div class="preview-label">KEEPER / 0.2　·　界面预览，示例数据</div></div>`
  : windowName === "widget"
    ? widget()
    : windowName === "settings"
      ? ""
      : panel();

document.addEventListener("click", async (event) => {
  const b = event.target.closest("button");
  if (!b) return;
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
    document.documentElement.dataset.theme = b.dataset.theme;
    document
      .querySelectorAll("[data-theme]")
      .forEach((el) => el.classList.toggle("active", el === b));
    return;
  }
  if (b.dataset.pick) {
    const { pick, value } = b.dataset;
    b.closest("details")?.removeAttribute("open");
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
    const open = $("details[open]");
    if (open) {
      open.removeAttribute("open");
      open.querySelector("summary").focus();
    } else
      action(windowName === "settings" ? "close-settings" : "close-detail");
  }
  if (event.key === "Enter" && event.target.id === "widget") action("detail");
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
        proxyUrl: form.get("proxyUrl"),
        widgetFont: form.get("widgetFont"),
        pollSeconds: Number(form.get("pollSeconds")),
        rememberPassword: form.has("rememberPassword"),
        allowPrivateHttp: form.has("allowPrivateHttp"),
        autoStart: form.has("autoStart"),
      },
      clearPassword: form.has("clearPassword"),
    });
    $("#settings-error").textContent = preview
      ? "预览模式：未写入注册表。"
      : "";
  } catch (error) {
    $("#settings-error").textContent = String(error);
  } finally {
    save.disabled = false;
    save.innerHTML = `保存并连接 ${icon("arrow")}`;
  }
});
if ($("#widget")) {
  let down = null,
    dragging = false;
  $("#widget").addEventListener("pointerdown", (event) => {
    if (event.button === 0) {
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
      action("drag");
    }
  });
  $("#widget").addEventListener("pointerup", () => {
    if (down && !dragging) action("detail");
    down = null;
  });
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
try {
  state.settings = await api.call("get_settings");
  applyAppearance();
  if (windowName === "settings") {
    if (preview) $(".preview-panel").innerHTML = settings();
    else root.innerHTML = settings();
  } else {
    await api.on("sample", (s) => {
      state.sample = s;
      state.error = "";
      updateSample();
    });
    await api.on("connection-error", (error) => {
      state.error = String(error);
      updateSample();
    });
    await api.on("detail-open", () => openDetail());
    await api.on("detail-close", () => {
      state.visible = false;
      state.generation++;
    });
    state.sample = await api.call("last_sample");
    updateSample();
    if ($("#filters")) {
      const keys = await api
        .call("get_view", { view: "keys", query: { range: "today" } })
        .catch(() => ({}));
      state.keys = keys.options || [];
      renderFilters();
      await load();
    }
  }
  await api.on("configured", async (s) => {
    state.settings = s;
    state.sample = null;
    state.error = "";
    state.keys = [];
    applyAppearance();
    updateSample();
    if ($("#filters")) {
      state.keys =
        (
          await api
            .call("get_view", { view: "keys", query: { range: "today" } })
            .catch(() => ({}))
        ).options || [];
      renderFilters();
    }
  });
  await api.on("settings-open", async () => {
    if (windowName === "settings") {
      state.settings = await api.call("get_settings");
      applyAppearance();
      root.innerHTML = settings();
    }
  });
} catch (error) {
  if ($("#content"))
    $("#content").innerHTML = empty("请在桌面应用中打开", String(error), false);
}

if (windowName === "widget" || (preview && windowName !== "settings")) {
  const poll = async () => {
    const started = performance.now();
    if (state.settings.endpoint && !document.hidden) {
      try {
        state.sample = await api.call("sample");
        state.error = "";
      } catch (error) {
        state.error = String(error);
      }
      updateSample();
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
