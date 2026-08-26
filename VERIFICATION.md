# 验证记录 · 0.3.0

## 本地检查

- 前端生产构建成功；开发示例与注入测试接口不进入生产包。
- 9 项 JavaScript 单元测试通过，包括 16 秒零用量暂留、重复采样去重、单方向新增、断线 / 基线 / 重置清空及不同采样间隔。
- Chromium 检查五个指标页、四个账户子页、筛选、空 / 断线 / 长文本、代理及字体设置；实际 640 × 640 详情布局、主指标统一 26px / 700、浅深主题主要文字对比度至少 4.5:1。
- 配额 564,617,884 完整单行且普通配额表无需横向滚动；宽请求表单独横向滚动，不撑宽面板。
- 悬浮块在 100% / 125% / 150% / 200% 浏览器像素比例下检查边界及长数字。示例截图仅用于布局证据。
- 16 秒暂留的界面集成测试确认详情保持真实 0；断线清空悬浮块增量。静态文字禁选，输入框仍支持全选。
- 浏览器和临时开发服务器在测试 finally 中关闭。

## Windows 构建

- 2026-08-26 19:45（北京），[Windows CI](https://github.com/wlyzqm/Keeper-UsagePanel/actions/runs/32964062713) 构建成功。程序源码提交为 `0711a6c81a623bd617c97fab8750a1526296cbe3`，版本标签 `v0.3.0` 指向此提交。后续提交仅涉及发布流程和交付文档。
- Windows runner 通过 9 项 JavaScript 检查、13 项 Rust 核心检查及 1 项 Windows DPAPI 往返检查；Tauri 发布编译及 NSIS 打包成功。
- 下载的 CI ZIP SHA256 与 GitHub artifact digest 一致，ZIP CRC 检查通过。便携程序为 Windows x64 GUI PE。安装器外壳为 NSIS x86，内含的应用为 x64。
- 本地交付含独立 EXE、安装 EXE、带说明 / 截图 / 字体许可的便携 ZIP，以及 SHA256SUMS.txt；ZIP 中的 EXE 与 CI 原件完全一致。

| 文件 | 字节数 | SHA256 |
| --- | ---: | --- |
| KeeperUsagePanel.exe | 10,149,376 | `69a36dcf144b0f020ecd94484275da9eec66e7cc46cb5501aae4734301e84d2b` |
| KeeperUsagePanel_0.3.0_x64-setup.exe | 2,761,260 | `f3e96271d4d7e00c9d1f7fe0a414a4d1edc33b160e6c5d5d6d88ce01cf9c0463` |

## Release 发布状态

已新增主分支构建成功后自动发布的工作流。发布步骤先上传完整附件，再公开 Release；同版本不覆盖其他源码的产物。

首轮发布在查询不存在标签时收到 GitHub 422，已修复为先判断标签引用是否存在；已创建指向上述构建源码的 v0.3.0 标签。修复后的新标签 / 已有标签分支通过本地模拟检查。当前连接器重跑 Actions 返回 403（无 Actions 写权限），需在 [发布流程](https://github.com/wlyzqm/Keeper-UsagePanel/actions/runs/32964987960) 点击 Re-run failed jobs，或手动运行 Publish Windows release 并填写 build run ID `32964062713`。此时尚未确认公开 Release，不能把本地成品等同于已发布附件。

## 证据边界

未在 Windows 桌面人工运行本轮应用。注册表配置完整流程、置顶、悬停跨窗、拖动吸附、托盘、真实多屏 DPI、安装器和连接真实 Keeper 的代理网络需要使用本轮 EXE 按 WINDOWS-ACCEPTANCE.md 验收。CI 编译及 DPAPI 单元测试不替代这些交互验收。本轮不访问真实 Keeper 凭据、不改远端部署。
