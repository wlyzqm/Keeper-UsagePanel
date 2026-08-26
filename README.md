# Keeper UsagePanel

Windows 10/11 置顶悬浮球，**直接调用已有 Keeper API**。不安装远端服务、不读写远端数据库、不改变 Keeper / CPA / Usage-Sync 部署。

初版 `0.1.0`，接口兼容基线：Keeper `1.14.8`。

## 使用

1. 运行 `KeeperUsagePanel.exe`（Windows x64 自包含版，无需另外安装 .NET，无需管理员权限）。
2. 首次启动填写 **Keeper 完整地址**，例如 `https://keeper.example/usage`。不预置服务器地址；应使用这台 Windows 已能访问的 Keeper 地址。
3. Keeper 启用认证时，填写 **Keeper 登录密码**，不是 sk。可选择记住密码。
4. 保存后自动连接；以后启动读取当前用户注册表。右键悬浮球或通过托盘可以重新设置。

公网使用 HTTPS。若使用受保护的专网 HTTP，需在设置中明确勾选允许；不要将密码经不可信明文网络传输。服务器自己的 `127.0.0.1` 地址不能直接当作 Windows 访问地址。

配置保存位置：`HKEY_CURRENT_USER\Software\KeeperUsagePanel`。包括地址、刷新间隔、窗口位置和偏好。记住密码时，`ProtectedPassword` 是 Windows 当前用户 DPAPI 加密的二进制值，**不保存明文密码**。会话 Cookie 仅在内存中使用。不需要连接 JSON 文件。

## 悬浮球

- 北京时间今日全部 Key 的 Token 总量。
- 相邻两次**成功采样**之间的输入 / 输出 Token 增量，默认 2 秒刷新；它是这一段时间的数量，不是 Token/s。
- 健康、波动、异常、安静等结论。断线显示离线，不沿用旧的健康标签。
- 首次采样显示 `—` 并建立基线；下一次开始显示增量。无新增用量显示 `+0`。
- 失败采样不更新基线；恢复后显示整个实际间隔的增长。跨午夜补读上一日期末值，再与新日期累计合并。累计回退则重新建立基线，不显示负数。

计算示例：前一次累计输入 1,000、输出 100，2 秒后累计输入 1,500、输出 160，显示 `入 +500 / 出 +60`。

此增量来自 Keeper 已记录的日累计变化，不是模型流式生成直播。回填到已经关闭、且不在跨日补读范围内的更早日期，不会变成本轮实时用量。程序重启建立新基线，不把退出期间的历史一次性显示成短间隔增量。

健康取 Keeper 所有可见凭证类型返回的五小时健康数据，先合计成功 / 失败，再套用 Keeper 动态成功率阈值；不按 Key 分开，也不平均账户状态。健康覆盖 Keeper 当前可见账户，不能代表已经删除或无法归属的凭证。账户健康缓存约 10 秒，今日用量按配置间隔读取。

## 悬停面板

鼠标停留在球、面板或其下拉菜单时保持展开；离开后约 280ms 收起。从球跨入面板不会立即消失。球可拖动和贴边，面板自动向屏幕内侧展开；支持托盘隐藏、恢复、退出以及可选开机启动。

面板默认“今日 / 全部 Key”，支持昨日、近 7 天、近 30 天、本月和自定义日期。面板筛选不改变悬浮球的全局口径。

| 视图 | 内容 |
| --- | --- |
| 总览 | 请求数、成功失败、成功率、Token 总量与输入/输出/缓存/推理组成、缓存率、估算成本 |
| 成本 | 普通输入、缓存读、缓存写、输出成本；模型每请求成本、输出量与缓存率 |
| 延迟 | 首 Token 和请求总耗时的 P95 / 最大值，样本数与缺失说明 |
| 分布 | 模型、Key、认证账户、提供商的请求数、Token、占比与成本表格 |
| 账户概览 | 累计请求/Token/成功率/缓存率、身份信息、最近使用、五小时健康、当前配额与套餐 |
| 额度历史 | 主/次配额周期、额度变化、Token / 成本与每百分点效率 |
| 请求明细 | 日期 + Key + 账户筛选，游标分页，Token 分项、结果、成本与延迟 |
| 错误事件 | Keeper 返回的错误摘要、HTTP / 错误码、可重试与重试时间，游标分页 |

仅显示单值指标和表格，不显示曲线、散点、热图或趋势线。

### 统计边界

- 输入包含缓存，输出包含推理；这些子项不重复加到总 Token。
- 缓存率为汇总缓存读取 / 汇总输入，分母为零显示 `—`。
- 成本沿用 Keeper 定价，是 API 等价估算；缺价格不显示为零，订阅同步不代表实际扣费。
- 账户累计概览、当前额度、配额周期不受外层日期 / Key 筛选影响，界面会注明。
- 错误 API 不支持日期或 Key 筛选。客户端对当前游标页按日期筛选，只显示**本页条数**，不冒充日期范围总数；需要更早记录时翻页。错误事件与失败请求数不能相加。
- 延迟诊断仅支持 Keeper 最近 30 天有效样本，零 / 缺失延迟不是瞬时完成。
- 默认今日口径要求 Keeper 配置 `Asia/Shanghai`。Windows 系统时区可以不同，展示仍使用北京时间。
- Keeper 保存多少历史，就显示多少；不补出未观察到的额度历史。

## 构建和测试

需要 .NET 10 SDK。核心测试不依赖 Windows；界面运行只支持 Windows。

```sh
dotnet run --project desktop/KeeperUsagePanel.Core.Tests -c Release
dotnet build desktop/KeeperUsagePanel/KeeperUsagePanel.csproj -c Release
dotnet publish desktop/KeeperUsagePanel/KeeperUsagePanel.csproj -c Release -r win-x64 --self-contained true -p:PublishSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true -p:DebugType=None -p:DebugSymbols=false -o dist/windows-x64
```

GitHub Actions 在 Windows runner 上执行核心测试、编译并生成 EXE artifact。EXE 不包含任何服务器地址、密码或凭据。初版未代码签名，Windows 可能提示未知发布者。

## 验收状态

见 [VERIFICATION.md](VERIFICATION.md) 和 [Windows 验收清单](WINDOWS-ACCEPTANCE.md)。Linux 上的成功编译、模拟接口测试和 HK 本机 API 验证，均不等同于 Windows 窗口行为或 Windows 到 Keeper 网络链路已验收。
