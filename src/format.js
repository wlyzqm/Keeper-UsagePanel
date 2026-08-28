export const escape = (value) =>
  String(value ?? "").replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ],
  );
export const number = (v) =>
  v == null || !Number.isFinite(Number(v))
    ? "—"
    : Number(v).toLocaleString("en-US", { maximumFractionDigits: 0 });
export function compact(v) {
  if (v == null) return "—";
  const n = Number(v);
  for (const [size, suffix] of [
    [1e18, "E"],
    [1e15, "P"],
    [1e12, "T"],
    [1e9, "B"],
    [1e6, "M"],
    [1e3, "K"],
  ])
    if (n >= size)
      return (
        (n / size)
          .toFixed(n / size >= 100 ? 0 : n / size >= 10 ? 1 : 2)
          .replace(/(\.\d*?[1-9])0+$|\.0+$/, "$1") + suffix
      );
  return number(n);
}
// The docked widget only has room for three numeric glyphs on one line.
// Keep the magnitude on a separate line so the health signal remains legible.
export function edgeCompact(v) {
  if (v == null || !Number.isFinite(Number(v))) return { value: "—", unit: "" };
  const n = Math.max(0, Number(v));
  const scales = [
    [1e3, "K"],
    [1e6, "M"],
    [1e9, "B"],
    [1e12, "T"],
    [1e15, "P"],
    [1e18, "E"],
  ];
  let index = scales.findLastIndex(([size]) => n >= size);
  if (index < 0) return { value: String(Math.round(n)), unit: "" };
  let [size, unit] = scales[index];
  let scaled = n / size;
  if (scaled >= 999.5 && index < scales.length - 1) {
    [size, unit] = scales[++index];
    scaled = n / size;
  }
  const value =
    scaled >= 9.95 ? scaled.toFixed(0) : scaled.toFixed(1).replace(/\.0$/, "");
  return { value, unit };
}
export const percent = (a, b) =>
  a != null && b > 0 ? `${((a / b) * 100).toFixed(1)}%` : "—";
export const duration = (v) => (v > 0 ? `${(v / 1000).toFixed(2)} s` : "—");
export const money = (v, available = true) =>
  available && v != null
    ? "$" +
      Number(v).toLocaleString("en-US", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 4,
      })
    : "—";
export const cost = (obj, key = "cost_usd", availability = "cost_available") =>
  money(obj?.[key], obj?.[availability] === true);
export const time = (v) =>
  v && Number.isFinite(Date.parse(v))
    ? new Intl.DateTimeFormat("zh-CN", {
        timeZone: "Asia/Shanghai",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false,
      }).format(new Date(v))
    : "—";
export const day = () =>
  new Intl.DateTimeFormat("en-CA", {
    timeZone: "Asia/Shanghai",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(new Date());
export const tone = (s) =>
  ({
    健康: "green",
    波动: "amber",
    异常: "red",
    安静: "neutral",
    离线: "neutral",
    未连接: "neutral",
  })[s] || "neutral";

export const widgetFontStack = (font) =>
  [
    font?.trim() || "HarmonyOS Sans SC",
    "HarmonyOS Sans SC",
    "Microsoft YaHei UI",
    "Microsoft YaHei",
    "Noto Sans CJK SC",
  ]
    .filter((f, i, a) => a.indexOf(f) === i)
    .map((f) => '"' + f.replace(/["\\]/g, "") + '"')
    .join(",") + ",sans-serif";
