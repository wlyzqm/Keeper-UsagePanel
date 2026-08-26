# 验证记录 · 0.4.2

## 依据与边界

本轮按 Keeper 1.14.8 源码 `c0129050def1f3e980ed68492014326d35710917` 核对登录、角色中间件、Key owner 选项、用量汇总及 sk 限流接口。只读检查确认远端容器仍为 1.14.8；未读取真实登录凭据，未改变远端服务。

实现使用本地协议样例与 Chromium 预览验证，不等同于真实 Keeper 联调或 Windows 桌面人工验收。长时间运行、密码跨应用复制、多屏与托盘须按 WINDOWS-ACCEPTANCE.md 用本轮 EXE 验收。

## 本地检查

- 9 项 JavaScript 单元检查已通过，覆盖格式及 16 秒增量暂留。
- 前端生产构建已通过，生产包不含预览样例 / 测试入口。
- Chromium 已验证 sk 只读页签 / 日期、无管理员请求；管理员 Key owner 同步悬浮块、重开保留选择、丢弃旧范围采样、点击入口和普通鼠标样式；新增登录方式与密码输入全选。
- Rust 核心 19 项测试已通过，涵盖真实本地 HTTP / SOCKS 协议、登录后角色校验、sk 请求路由白名单、跨日增量、Key 切换基线和近五小时请求健康、无效凭据不循环登录。
- 本轮浏览器回归覆盖原有主题 / 指标页 / 账户页 / 溢出 / 像素比例及 16 秒暂留；测试后关闭浏览器与临时服务器。
- [Windows 构建](https://github.com/wlyzqm/Keeper-UsagePanel/actions/runs/32969167945) 已成功：9 项 JavaScript、19 项 Rust 核心及 2 项桌面测试（DPAPI 往返、悬停状态机）通过；Tauri 与 NSIS 编译成功。
- 构建源码为 `19c1c06002eb0c2cdb5083dcfb021dbd5b6a7722`；`v0.4.2` 标签指向该提交，后续文档提交不改变程序。
- [GitHub Release](https://github.com/wlyzqm/Keeper-UsagePanel/releases/tag/v0.4.2) 已公开发布四个附件，独立 EXE、安装 EXE、便携 ZIP 与 SHA256SUMS 均已下载校验。文件大小和 SHA256 与 GitHub asset digest 一致；ZIP CRC 通过，ZIP 内 EXE 与独立 EXE 完全一致。
- EXE 为 x64 GUI PE，包含 0.4.2 版本信息。程序未签名；本地交付目录 `dist/` 保存同一批 Release 原件。

## 窗口补修与版本

0.4.0 推送后在继续追踪依赖和窗口状态时，确认原有 `settings.is_visible()` 门控会让仍打开的后台设置窗阻止悬停。0.4.1 改为只按设置窗是否处于前台暂停自动展开，显式点击不经过此门控。取消设置窗置顶、重复打开保留输入、原生显隐统一仍保留。

Tao 0.35.3 的 `is_visible()` 本身调用 `IsWindowVisible`，不能把长期悬停问题直接归因于该 getter 的缓存。内部显隐 flags 与原生 ShowWindow 混用是另一类状态风险；本轮统一浮窗显隐方式，但 Windows 长时故障是否完全消除仍需实机验证。

## 0.4.2 控制台入口

按 Keeper 1.14.8 的 `router.go` 与 `UsagePage.tsx` 追踪 CPA 链接：管理员 `status.cpa_public_url` + `management.html`；字段缺失使用当前源。sk 不开放 status，因此禁用 CPA 按钮且 Rust 同样拒绝此调用。新增协议测试确认管理员路由及 sk 不发请求，纯函数测试覆盖域名 / 端口 / 子路径和不安全链接拒绝。外部链接只允许 HTTP / HTTPS，通过 Windows 默认浏览器打开，不传递桌面登录凭据。

Chromium 顶栏检查确认两个入口的调用目标、sk 下 CPA 禁用、连接状态和按钮顺序，以及实际 640px 下无横向溢出；浅深主题截图已更新。

## 发布文件

| 文件 | 字节数 | SHA256 |
| --- | ---: | --- |
| KeeperUsagePanel-0.4.2-win-x64.zip | 4,076,209 | `1c59569bb414b2540e7a713542c6d39c4455124b7e25cc7cebaa50cd0bb5299b` |
| KeeperUsagePanel.exe | 10,381,824 | `b6843b470fd548809612dbcb9a69ae17e30d0a6434fea6820596b9a60c2e5365` |
| KeeperUsagePanel_0.4.2_x64-setup.exe | 2,809,128 | `2c594141512d7c61761012d449bb0056959370af6ca9dbeb3f7410a31d2cb79b` |
| SHA256SUMS.txt | 291 | `150054e8d354ee5c8b04809a48eee1627fb9e752b27ac9c17ce9f38fd64e96ce` |

Windows 默认浏览器跳转、长时间悬停、跨应用复制密码及真实 sk 登录仍需按验收清单实机确认；未用自动测试代替这些人工交互结果。
