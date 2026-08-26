# 0.1.0 验证记录

日期：2026-08-26，北京时间。最终结构为 **WPF EXE 直连 Keeper**，无远端适配服务。

## 实际 Keeper 接口

17:43 在 HK 机本机访问现有 Keeper 1.14.8，登录后执行只读业务接口并退出测试会话。下列接口均返回 200（登录/退出为 204）：

- usage/activity?window=today：今日输入、输出、总 Token；确认窗口起点为北京时间零点，连续两次查询无新增时增量为 0。
- usage/identities/page：全类型五小时健康、账户分页。
- usage/api-keys/options：返回 3 个 Key 选项。
- usage/overview：全局与单 Key 查询。
- usage/analysis?range=30d：Token 分项、成本拆分、模型效率与分布字段齐全。
- usage/analysis/latency：延迟诊断。
- quota/cache：只读取已有缓存，没有触发刷新。
- quota/history：主额度历史。
- usage/events：指定账户的请求明细。
- usage/identities/:id/errors：账户错误事件。

观测到 1 个账户。HK 本机 API 单次约 1–72 ms；这不是 Windows 客户端端到端延迟指标。本次 API 快速采样的输入/输出增量均为零，不宣称已用真实新增记录完成 Windows 端增量验收。

远端部署路线已取消：仅曾创建一个空目录，未上传文件、未安装或启动新服务；空目录已删除。原有 Keeper / CPA / Usage-Sync 未修改。

## 自动检查

核心测试覆盖直接 Keeper 路由、登录会话、相邻累计差值、跨日补偿、无新增、失败不更新基线、累计回退、健康阈值、Key 筛选、错误分页日期边界、北京时间与数字格式。

最终本地检查：24 项核心检查通过；WPF Release 编译 0 警告、0 错误；Windows x64 自包含单文件发布成功，产物识别为 PE32+ Windows GUI x86-64。EXE 约 166 MiB，包含 .NET 桌面运行时；压缩包更适合传输。

源码包含 Windows CI 工作流；尚未宣称 GitHub Actions 执行成功。Linux 编译不能证明 Windows UI 实际运行效果。

## 尚待 Windows 实机验收

首次配置窗口、注册表落盘 / DPAPI、置顶与鼠标焦点、混合 DPI / 多屏、休眠恢复、Windows 到 Keeper 的实际连接。使用 [WINDOWS-ACCEPTANCE.md](WINDOWS-ACCEPTANCE.md) 验收。
