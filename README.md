# Keeper UsagePanel 0.4.2

面向 Windows 10 / 11 x64 的 Keeper 置顶悬浮球。Tauri 2 + Rust + HTML/CSS，直接连接既有 Keeper，不安装远程适配服务，不启动本地 HTTP 服务。

![浅色界面，示例数据](docs/overview-light.png)

## 使用

[Windows Release 下载入口](https://github.com/wlyzqm/Keeper-UsagePanel/releases/latest)。本轮构建及发布状态见 [验证记录](VERIFICATION.md)。正式发布包含安装 EXE、独立便携 EXE、便携 ZIP 和 SHA256 校验文件。

推荐运行 `KeeperUsagePanel_0.4.2_x64-setup.exe` 安装包。缺少 WebView2 时，安装器联网下载 Microsoft 运行时。已有 WebView2 的电脑也可直接运行便携版 `KeeperUsagePanel.exe`，不需要 .NET。

升级前请先退出旧版。安装包和程序未做代码签名，可使用交付目录中的 `SHA256SUMS.txt` 核对文件。

首次启动填写完整 Keeper 地址，例如 `https://keeper.example/usage`，选择「管理员密码」或「API Key（sk）」登录。保存前验证登录。配置写入 `HKCU\Software\KeeperUsagePanel`，兼容 0.1.0 的地址、密码和位置配置。

- 紧凑圆角悬浮块：200 × 58 DIP，左侧状态点，中间北京时间今日 Token 总量，右侧输入 / 输出。
- 输入 / 输出：上次成功采样到本次成功采样的增量，默认 2 秒，不是每秒速率。两者同时为 0 时暂留上一组结果；连续零用量累计达到 16 秒后归零。有任一路新增就显示本次完整一组并重新计时。首采样、计数重置或断线显示 —。详情始终显示真实本次采样值。
- 状态点表示健康类别；悬停提示和详情显示“健康”“波动”等结论，断线明确显示离线。
- 悬停展开详情，离开两窗后收起；球与面板间保留穿越区域。拖动移动，靠近边缘时吸附并记住位置。
- 点击悬浮块也能展开（保留悬停入口）；普通鼠标样式不变，实际拖动时才显示拖动样式。设置窗不置顶，可独立切换到其他应用复制密码；右键或托盘可设置、隐藏、退出。详情 Esc 收起，设置 Esc 关闭。
- 今日 / 近 7 天 / 近 30 天 / 昨日 / 本月 / 自定义日期，管理员可选 Key owner；面板和悬浮块同步到同一个 Key。日期范围仅改变详情，悬浮块仍显示北京时间今日。重新连接或重启后管理员默认全部 Key。
- 蓝色高对比配色，主指标统一 26px。静态文字不可选中；输入框保留选择、复制、粘贴。数字与时间不换行，宽明细表在表格内部横向滚动。
- 管理员全局模式提供总览、成本、延迟、用量分布，以及认证账户的概览、额度历史、请求明细和错误事件。只显示数值、表格与事件摘要，不显示图表。

## sk 与 Key owner 权限

- sk 使用 Keeper 的 `auth/api-key-login`，随后验证 `auth/session` 的 `api_key_viewer` 角色；只调用 `key-overview` 和 `key-activity`。显示自身请求数、Token 组成、缓存率、总成本及请求健康；不显示管理页签和 Key 切换器。
- 管理员通过 `usage/api-keys/options` 的别名 / 脱敏标签选择 Key owner。共享范围由 Rust 管理；切换即清空旧采样和 16 秒暂留，旧范围返回不能覆盖新范围，下一次成功采样重新建立基线。
- 权限在 Rust 请求层再次限制，不能通过隐藏页签或 IPC 参数访问管理员指标。sk 不接受其他 Key ID；Keeper 关闭认证时 sk 模式拒绝意外返回的管理员身份。
- Keeper 对 sk 的 overview / activity 各限每秒一次；面板和悬浮块共享节流，忙碌时使用实际采样间隔。无效凭据停止自动登录重试，修改设置后重连。

## 控制台入口

浮窗标题栏按「连接状态 → 用量控制台 → CPA 控制台 → 设置」排列。设置改为醒目的长条文字按钮。

- 用量控制台在 Windows 默认浏览器打开设置中提供的 Keeper 地址。
- 管理员点击 CPA 控制台时读取 `/api/v1/status.cpa_public_url`，按 Keeper 的规则补上 `management.html`，支持完整地址、子路径与裸域名。Keeper 未设置该字段时，与 Keeper 网页一样使用当前源的 `/management.html`。
- `status` 是管理员接口；sk 模式保留禁用的 CPA 按钮并解释权限限制，不请求管理接口或猜测另一个 CPA 域名。
- 只允许 HTTP / HTTPS 链接，不传递应用内的密码、sk 或会话 Cookie；浏览器与桌面应用各自登录。

## 代理与字体

设置中展开「代理设置」，留空表示直连，不继承环境变量代理。

- `http://127.0.0.1:7890`、`https://proxy.example:8443`
- `socks5://127.0.0.1:1080`（客户端解析目标 DNS）
- `socks5h://127.0.0.1:1080`（代理解析目标 DNS）
- 需要认证时：`scheme://用户名:密码@主机:端口`，特殊字符需 URL 编码。所有 Keeper 请求使用同一个代理，不失败回退直连。

悬浮球字体可填本机字体名称。默认 `HarmonyOS Sans SC`；不存在时依次回退 `Microsoft YaHei UI`、`Microsoft YaHei`、`Noto Sans CJK SC`、系统 sans-serif。自定义字体缺失时先回退鸿蒙黑体，再走上述序列。不内置或下载鸿蒙字体。详情的拉丁数字使用随包的 Manrope，中文使用系统字体。

## 配置与安全

`Endpoint`、`AuthMode`、`PollSeconds`、`RememberPassword`、`AllowPrivateHttp`、`AutoStart`、`Theme`、`WidgetFont`、`X`、`Y` 保存为注册表值。可选登录凭据（密码或 sk）`ProtectedPassword` 和代理地址 `ProtectedProxyUrl` 使用 Windows DPAPI 当前用户加密。

不把登录密码或完整 sk 返回给前端；设置显示「已保存，留空继续使用」；仅在登录方式与 Keeper 地址均未改变时复用旧凭据。会话 Cookie 只驻留 Rust 内存。HTTPS 校验证书，不关闭证书校验。非本机 HTTP 需显式勾选专网确认。界面仅加载内置资源；开发用示例数据不会打入 EXE。图标使用用户提供的 Keeper SVG。

## 统计边界

- 适配 Keeper 1.14.8 的 `/api/v1` 接口。Keeper 时区需为 `Asia/Shanghai`。Windows 的本地时区不影响北京时间口径。
- 增量从相邻成功采样的今日累计值做差；失败不更新基线。跨午夜读取旧日期的 analysis 汇总（sk 使用 key-activity）接续，无法接续或计数倒退时重新建立基线。重启不回放历史增量。
- 今日之外的详情使用 overview / analysis 的日期范围；缓存和推理属于子项，不重复计入 Token 总量。
- 管理员全局健康聚合 Keeper 凭据健康五小时窗口；所选 Key / sk 模式改用该 Key 的近五小时请求成功与失败数，并复用 Keeper 的阈值公式。两者统计对象不同，详情明确标注。
- 成本是 Keeper 价格配置的 API 等价估算，不等于订阅真实扣费；价格缺失显示 —。
- 认证账户指标仅在管理员全部 Key 下显示：额度、额度历史与错误事件没有可靠的 Key 归属，不把共享数据标成某个 Key 的数据。账户概览是累计统计。只读配额缓存，不触发上游配额刷新。
- 请求明细按日期、Key、账户筛选。错误接口不支持日期 / Key 查询，因此日期在当前游标页内筛选，明确显示本页条数；不能视为日期范围总数。
- 断线后的下一次成功增量可能覆盖较长间隔，显示实际秒数。历史回填到已关闭日期不保证包含在实时增量中。

## 开发

### Windows 本机构建前置条件

只运行成品 EXE 无需安装 Rust 或 C++ 编译工具；下面仅用于从源码构建。

1. 安装 [Microsoft C++ 生成工具](https://visualstudio.microsoft.com/visual-cpp-build-tools/)，勾选「使用 C++ 的桌面开发」，保留 MSVC x64/x86 工具与 Windows SDK。
2. 安装 Rust（包含 Cargo）。可在 PowerShell 执行 `winget install --id Rustlang.Rustup -e`，没有 winget 时使用 [Rust 官方安装器](https://www.rust-lang.org/tools/install)。
3. 安装完成后关闭并重新打开终端；若使用 VS Code 集成终端，重启 VS Code。确认 WebView2 运行时已安装。
4. 在新的 PowerShell 中选择 MSVC 工具链并检查 Cargo：

```powershell
rustup default stable-x86_64-pc-windows-msvc
cargo --version
rustc --version
```

若报 `cargo metadata ... program not found`，说明当前终端找不到 Cargo，还未进入项目编译。`npm ci` 不会安装 Rust。可先检查默认安装位置：

```powershell
Test-Path "$env:USERPROFILE\.cargo\bin\cargo.exe"
```

若返回 `True`，可为当前 PowerShell 临时补充 PATH 后重试：

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo --version
```

若仍无法识别，检查 Rust 安装器是否成功完成，以及是否自定义过 `CARGO_HOME`；不要通过重装 npm 依赖处理此错误。完整环境要求见 [Tauri 官方前置要求](https://v2.tauri.app/zh-cn/start/prerequisites/)。

### 项目命令

```sh
npm ci
npm run dev          # 前端
npm run tauri dev    # Windows 桌面开发
npm test
cargo test -p keeper-core
npm run test:ui      # CHROMIUM_PATH 可指定浏览器
npm run tauri build -- --target x86_64-pc-windows-msvc
```

浏览器开发预览：`http://127.0.0.1:1420/?preview=1`，加 `role=sk` 预览受限角色；加 `theme=dark`、`state=empty|offline|long`、`window=settings` 检查界面状态。加 `standalone` 按独立窗口尺寸预览；`window=widget&standalone` 仅预览悬浮块。预览显式标注示例数据，不连接真实 Keeper，不写注册表。

Linux 交叉编译可使用 `bash scripts/build-linux.sh`，需要 Rust Windows MSVC target、LLVM、cargo-xwin 0.23+、NSIS。默认使用 clang 与预制 Windows sysroot，限制为单编译任务，并降低生成式 Windows 绑定库的优化级别，减少小内存机器的构建开销。GitHub Actions 使用 Windows runner 构建安装版与便携版。主分支构建成功后，发布流程按 package.json 版本创建 Release，标签指向实际构建提交；先上传完整附件再公开发布。PR 构建不发布。同一版本不覆盖不同源码的成品，修改程序后需升级版本号；可手动指定成功的主分支 build run ID 补发。

见 [Windows 验收清单](WINDOWS-ACCEPTANCE.md)、[验证记录](VERIFICATION.md)、[设计说明](docs/DESIGN.md)。
