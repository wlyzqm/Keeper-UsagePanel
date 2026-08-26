# 验证记录 · 0.2.0

## 本轮已完成

- 前端生产构建成功；示例数据及预览脚本只在 Vite 开发模式启用。
- Chromium 实际渲染浅色 / 深色、五个指标页、四个账户子页、日期 / Key 筛选、空状态、断线、长名称、长错误文本与设置页；脚本结束关闭浏览器和服务器。
- 5 项 JavaScript 格式化与字体回退检查通过。
- 13 项 Rust 核心检查通过：相邻采样增量、零增量、跨午夜接续、重置、64 位精度、健康阈值、登录及 Cookie、失败采样保留基线、日期 / Key 传递、账户只读接口、错误游标页范围、HTTP 代理路由、SOCKS5H 实际握手与代理端 DNS、代理地址校验。
- Windows runner 已通过上述核心检查，以及当前 Windows 用户的 DPAPI 加密 / 解密往返检查；桌面代码已通过 Windows 编译检查。
- 用户指定 SVG 已用于界面及 Windows 图标。字号调整后在 200 × 84 DIP 组件内检查无横向溢出。

## Windows 构建

- 2026-08-26 18:58（北京），[Windows CI](https://github.com/wlyzqm/Keeper-UsagePanel/actions/runs/32960071438) 全部成功，发布构建生成便携 EXE 和 NSIS 安装 EXE。
- 对应程序源码提交：`922850022486c03379b03c49b436dbf628c6478d`。后续提交仅补充交付文档。
- 下载的 CI ZIP 的 SHA256 与 GitHub 公布的 artifact digest 一致，ZIP CRC 检查通过。便携程序为 Windows x64 PE。
- 便携 EXE：10,149,376 字节；安装 EXE：2,764,987 字节。另提供含说明、截图和许可文件的便携 ZIP。

| 文件 | SHA256 |
| --- | --- |
| KeeperUsagePanel.exe | `1d16356d5f4931ba16424daad1a508bdc39758d0ab2355b6cff88c855d600ae1` |
| Keeper UsagePanel_0.2.0_x64-setup.exe | `5cbe5df60b8d5d5eeff22297505f3f2d4debe3093707aeae2a1f19159e5e794e` |

交付目录的 `SHA256SUMS.txt` 另含便携 ZIP 校验值。本机 Linux 交叉编译因 Windows 绑定库编译的内存压力而停止，没有把未完成的交叉编译记作成功；临时 SDK 缓存已清理。

## 证据边界

浏览器预览使用明确标注的示例数据，不代表 Windows 原生窗口已运行。注册表配置读写及凭据保存恢复、置顶、悬停跨窗、拖动吸附、托盘、多屏 DPI、安装器和连接真实 Keeper 的代理网络仍需运行 EXE 按 WINDOWS-ACCEPTANCE.md 验收。DPAPI 单元测试通过不替代完整配置流程的人工验收。

0.1.0 曾在 2026-08-26 17:43（北京）只读验证 HK 机 Keeper 1.14.8 的真实接口。那是旧实现的接口证据，不作为本轮 Rust 客户端已完成真实连通验收的结论。本轮网络集成测试使用本机 HTTP / SOCKS 测试服务，无远程变更。本轮真实 Rust 登录验证因凭据读取未获授权而未执行，临时回环端口转发已关闭。
