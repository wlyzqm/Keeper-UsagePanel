# Keeper UsagePanel 0.2.0

面向 Windows 10 / 11 x64 的 Keeper 置顶悬浮球。Tauri 2 + Rust + HTML/CSS，直接连接既有 Keeper，不安装远程适配服务，不启动本地 HTTP 服务。

![浅色界面，示例数据](docs/overview-light.png)

## 使用

推荐运行 `Keeper UsagePanel_0.2.0_x64-setup.exe` 安装包。缺少 WebView2 时，安装器联网下载 Microsoft 运行时。已有 WebView2 的电脑也可直接运行便携版 `KeeperUsagePanel.exe`，不需要 .NET。

首次启动填写完整 Keeper 地址，例如 `https://keeper.example/usage`，以及 Keeper 登录密码（不是 sk）。保存前验证登录。配置写入 `HKCU\Software\KeeperUsagePanel`，兼容 0.1.0 的地址、密码和位置配置。

- 圆球：北京时间今日 Token，全局统计。
- 侧边胶囊：上次成功采样到本次成功采样的输入 / 输出增量，默认 2 秒。没有新增显示 0，建立基线或断线显示 —；不是每秒速率。
- 细环与短文字：健康、波动、异常、安静；连接故障明确显示离线。
- 悬停展开详情，离开两窗后收起；球与面板间保留穿越区域。拖动移动，靠近边缘时吸附并记住位置。
- 点击圆球也能展开；右键或托盘可设置、隐藏、退出。详情 Esc 收起，设置 Esc 关闭。
- 今日 / 近 7 天 / 近 30 天 / 昨日 / 本月 / 自定义日期，可按 Key 查看；不改变圆球的全局口径。
- 总览、成本、延迟、用量分布，以及认证账户的概览、额度历史、请求明细和错误事件。只显示数值、表格与事件摘要，不显示图表。

## 代理与字体

设置中展开「代理设置」，留空表示直连，不继承环境变量代理。

- `http://127.0.0.1:7890`、`https://proxy.example:8443`
- `socks5://127.0.0.1:1080`（客户端解析目标 DNS）
- `socks5h://127.0.0.1:1080`（代理解析目标 DNS）
- 需要认证时：`scheme://用户名:密码@主机:端口`，特殊字符需 URL 编码。所有 Keeper 请求使用同一个代理，不失败回退直连。

悬浮球字体可填本机字体名称。默认 `HarmonyOS Sans SC`；不存在时依次回退 `Microsoft YaHei UI`、`Microsoft YaHei`、`Noto Sans CJK SC`、系统 sans-serif。自定义字体缺失时先回退鸿蒙黑体，再走上述序列。不内置或下载鸿蒙字体。详情的拉丁数字使用随包的 Manrope，中文使用系统字体。

## 配置与安全

`Endpoint`、`PollSeconds`、`RememberPassword`、`AllowPrivateHttp`、`AutoStart`、`Theme`、`WidgetFont`、`X`、`Y` 保存为注册表值。可选登录密码 `ProtectedPassword` 和代理地址 `ProtectedProxyUrl` 使用 Windows DPAPI 当前用户加密。

不把登录密码返回给前端；设置显示「已保存，留空继续使用」。会话 Cookie 只驻留 Rust 内存。HTTPS 校验证书，不关闭证书校验。非本机 HTTP 需显式勾选专网确认。界面仅加载内置资源；开发用示例数据不会打入 EXE。图标使用用户提供的 Keeper SVG。

## 统计边界

- 适配 Keeper 1.14.8 的 `/api/v1` 接口。Keeper 时区需为 `Asia/Shanghai`。Windows 的本地时区不影响北京时间口径。
- 增量从相邻成功采样的今日累计值做差；失败不更新基线。跨午夜读取旧日期的 analysis 汇总接续，无法接续或计数倒退时重新建立基线。重启不回放历史增量。
- 今日之外的详情使用 overview / analysis 的日期范围；缓存和推理属于子项，不重复计入 Token 总量。
- 健康复用 Keeper 凭据健康的五小时窗口与阈值，聚合可见身份；删除或无法映射的身份不在该窗口覆盖内。
- 成本是 Keeper 价格配置的 API 等价估算，不等于订阅真实扣费；价格缺失显示 —。
- 账户概览是累计统计；额度及额度历史属于账户共享数据，不按日期 / Key 拆分。只读配额缓存，不触发上游配额刷新。
- 请求明细按日期、Key、账户筛选。错误接口不支持日期 / Key 查询，因此日期在当前游标页内筛选，明确显示本页条数；不能视为日期范围总数。
- 断线后的下一次成功增量可能覆盖较长间隔，显示实际秒数。历史回填到已关闭日期不保证包含在实时增量中。

## 开发

```sh
npm ci
npm run dev          # 前端
npm run tauri dev    # Windows 桌面开发
npm test
cargo test -p keeper-core
npm run test:ui      # CHROMIUM_PATH 可指定浏览器
npm run tauri build -- --target x86_64-pc-windows-msvc
```

浏览器开发预览：`http://127.0.0.1:1420/?preview=1`，加 `theme=dark`、`state=empty|offline|long`、`window=settings` 检查界面状态。预览显式标注示例数据，不连接真实 Keeper，不写注册表。

Linux 交叉编译可使用 `bash scripts/build-linux.sh`，需要 Rust Windows MSVC target、LLVM、cargo-xwin 0.23+、NSIS。默认使用 clang 与预制 Windows sysroot，限制为单编译任务，并降低生成式 Windows 绑定库的优化级别，减少小内存机器的构建开销。GitHub Actions 使用 Windows runner 构建安装版与便携版。

见 [Windows 验收清单](WINDOWS-ACCEPTANCE.md)、[验证记录](VERIFICATION.md)、[设计说明](docs/DESIGN.md)。
