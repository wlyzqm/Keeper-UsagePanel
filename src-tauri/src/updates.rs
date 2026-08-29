use crate::{settings, AppState};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    time::Duration,
};
use tauri::{Emitter, Manager, State, WebviewWindow};
use tokio::sync::Mutex as AsyncMutex;

const RELEASE_API: &str = "https://api.github.com/repos/wlyzqm/Keeper-UsagePanel/releases/latest";
const USER_AGENT: &str = "Keeper-UsagePanel-Updater";

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
    notes: String,
    release_url: String,
    portable: bool,
}

#[derive(Clone)]
struct Candidate {
    info: UpdateInfo,
    download_url: String,
    sha256: String,
}

#[derive(Default)]
pub struct UpdateState {
    candidate: Mutex<Option<Candidate>>,
    checking: AsyncMutex<()>,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    html_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Clone, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

fn version_parts(value: &str) -> Option<[u64; 3]> {
    let mut parts = value.trim().trim_start_matches('v').split('.');
    let result = [
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ];
    parts.next().is_none().then_some(result)
}

fn newer_than_current(version: &str) -> bool {
    version_parts(version)
        .zip(version_parts(env!("CARGO_PKG_VERSION")))
        .is_some_and(|(latest, current)| latest > current)
}

fn portable_mode() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return true;
    };
    let Some(parent) = exe.parent() else {
        return true;
    };
    let Some(local) = std::env::var_os("LOCALAPPDATA") else {
        return true;
    };
    let installed = PathBuf::from(local).join("Keeper UsagePanel");
    !(parent
        .to_string_lossy()
        .eq_ignore_ascii_case(&installed.to_string_lossy())
        && parent.join("uninstall.exe").is_file())
}

fn checksum_for(checksums: &str, asset_name: &str) -> Option<String> {
    checksums.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        let digest = fields.next()?;
        let name = fields.next()?.trim_start_matches('*');
        (name == asset_name
            && digest.len() == 64
            && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| digest.to_ascii_lowercase())
    })
}

async fn fetch_candidate(
    config: &settings::Settings,
    respect_skipped_version: bool,
) -> Result<Option<Candidate>, String> {
    let (http, _) = keeper_core::configured_http_client(
        &config.proxy_url,
        config.allow_invalid_certificates,
        120,
        10,
    )?;
    let response = http
        .get(RELEASE_API)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| format!("无法检查 GitHub Release：{error}"))?;
    if !response.status().is_success() {
        return Err(format!("GitHub Release 返回 HTTP {}", response.status()));
    }
    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|error| format!("无法解析 GitHub Release：{error}"))?;
    let version = release.tag_name.trim_start_matches('v').to_string();
    if !newer_than_current(&version)
        || respect_skipped_version && config.skipped_update_version == version
    {
        return Ok(None);
    }
    let portable = portable_mode();
    let asset_name = if portable {
        "KeeperUsagePanel.exe".to_string()
    } else {
        format!("KeeperUsagePanel_{version}_x64-setup.exe")
    };
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .ok_or_else(|| format!("Release 缺少更新文件 {asset_name}"))?;
    let checksum_asset = release
        .assets
        .iter()
        .find(|asset| asset.name == "SHA256SUMS.txt")
        .ok_or("Release 缺少 SHA256SUMS.txt")?;
    let checksums = http
        .get(&checksum_asset.browser_download_url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|error| format!("无法下载更新校验文件：{error}"))?;
    if !checksums.status().is_success() {
        return Err(format!("更新校验文件返回 HTTP {}", checksums.status()));
    }
    let checksums = checksums
        .text()
        .await
        .map_err(|error| format!("无法读取更新校验文件：{error}"))?;
    let sha256 = checksum_for(&checksums, &asset_name)
        .ok_or_else(|| format!("校验文件中缺少 {asset_name}"))?;
    Ok(Some(Candidate {
        info: UpdateInfo {
            version,
            notes: release
                .body
                .unwrap_or_else(|| "此版本未提供更新说明。".into()),
            release_url: release.html_url,
            portable,
        },
        download_url: asset.browser_download_url.clone(),
        sha256,
    }))
}

async fn check(
    app: &tauri::AppHandle,
    respect_skipped_version: bool,
) -> Result<Option<UpdateInfo>, String> {
    let updates = app.state::<UpdateState>();
    let _checking = updates.checking.lock().await;
    let config = app.state::<AppState>().settings.lock().unwrap().clone();
    let candidate = fetch_candidate(&config, respect_skipped_version).await?;
    let info = candidate.as_ref().map(|candidate| candidate.info.clone());
    *updates.candidate.lock().unwrap() = candidate.clone();
    app.emit("update-status", info.clone())
        .map_err(|error| format!("无法更新版本状态：{error}"))?;
    Ok(info)
}

pub fn track(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(12)).await;
        let _ = check(&app, true).await;
    });
}

#[tauri::command]
pub fn pending_update(updates: State<'_, UpdateState>) -> Option<UpdateInfo> {
    updates
        .candidate
        .lock()
        .unwrap()
        .as_ref()
        .map(|candidate| candidate.info.clone())
}

#[tauri::command]
pub async fn check_update(
    app: tauri::AppHandle,
    window: WebviewWindow,
) -> Result<Option<UpdateInfo>, String> {
    if !matches!(window.label(), "detail" | "settings") {
        return Err("请在面板或设置中检查更新".into());
    }
    check(&app, false).await
}

#[tauri::command]
pub fn skip_update(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    updates: State<'_, UpdateState>,
) -> Result<(), String> {
    let version = updates
        .candidate
        .lock()
        .unwrap()
        .as_ref()
        .map(|candidate| candidate.info.version.clone())
        .ok_or("当前没有可跳过的更新")?;
    {
        let mut config = state.settings.lock().unwrap();
        config.skipped_update_version = version;
        settings::save(&config)?;
    }
    *updates.candidate.lock().unwrap() = None;
    let _ = app.emit("update-status", Option::<UpdateInfo>::None);
    Ok(())
}

fn downloaded_path(version: &str, portable: bool, current: &Path) -> PathBuf {
    if portable {
        return current.with_file_name(format!(".KeeperUsagePanel-{version}-update.exe"));
    }
    std::env::temp_dir().join(format!("KeeperUsagePanel-{version}-setup-update.exe"))
}

#[tauri::command]
pub async fn install_update(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    updates: State<'_, UpdateState>,
) -> Result<(), String> {
    let candidate = updates
        .candidate
        .lock()
        .unwrap()
        .clone()
        .ok_or("当前没有可安装的更新")?;
    let config = state.settings.lock().unwrap().clone();
    let (http, _) = keeper_core::configured_http_client(
        &config.proxy_url,
        config.allow_invalid_certificates,
        300,
        10,
    )?;
    let response = http
        .get(&candidate.download_url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|error| format!("无法下载更新：{error}"))?;
    if !response.status().is_success() {
        return Err(format!("更新下载返回 HTTP {}", response.status()));
    }
    if response
        .content_length()
        .is_some_and(|size| size > 200 * 1024 * 1024)
    {
        return Err("更新文件超过 200 MB，已停止下载".into());
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("无法读取更新文件：{error}"))?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if digest != candidate.sha256 {
        return Err("更新文件 SHA-256 校验失败，未执行更新".into());
    }
    let current = std::env::current_exe().map_err(|_| "无法定位当前程序")?;
    let artifact = downloaded_path(&candidate.info.version, candidate.info.portable, &current);
    std::fs::write(&artifact, &bytes).map_err(|_| "无法写入临时更新文件")?;
    let helper = std::env::temp_dir().join(format!(
        "KeeperUsagePanel-update-helper-{}.exe",
        std::process::id()
    ));
    std::fs::copy(&current, &helper).map_err(|_| "无法创建更新助手")?;
    let mode = if candidate.info.portable {
        "portable"
    } else {
        "installed"
    };
    let mut command = Command::new(&helper);
    command
        .arg("--apply-update")
        .arg(std::process::id().to_string())
        .arg(mode)
        .arg(&artifact)
        .arg(&current);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command.spawn().map_err(|_| "无法启动更新助手")?;
    let _ = app.emit("update-status", Option::<UpdateInfo>::None);
    if let Some(client) = state.client.read().await.clone() {
        let _ = tokio::time::timeout(Duration::from_secs(1), client.logout()).await;
    }
    app.exit(0);
    Ok(())
}

#[cfg(windows)]
fn wait_for_parent(pid: u32) {
    unsafe {
        use windows_sys::Win32::{
            Foundation::CloseHandle,
            System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
        };
        let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if !process.is_null() {
            WaitForSingleObject(process, 120_000);
            CloseHandle(process);
        }
    }
}

#[cfg(not(windows))]
fn wait_for_parent(_: u32) {
    std::thread::sleep(Duration::from_secs(2));
}

fn apply_update_helper(args: &[OsString]) {
    let Some(pid) = args
        .get(2)
        .and_then(|value| value.to_str())
        .and_then(|value| value.parse().ok())
    else {
        return;
    };
    let Some(mode) = args.get(3).and_then(|value| value.to_str()) else {
        return;
    };
    let (Some(artifact), Some(target)) = (args.get(4), args.get(5)) else {
        return;
    };
    let artifact = PathBuf::from(artifact.as_os_str());
    let target = PathBuf::from(target.as_os_str());
    wait_for_parent(pid);
    if mode == "installed" {
        let _ = Command::new(&artifact).arg("/S").status();
    } else {
        let backup = target.with_file_name(".KeeperUsagePanel-update-backup.exe");
        let _ = std::fs::remove_file(&backup);
        if std::fs::rename(&target, &backup).is_ok() {
            if std::fs::rename(&artifact, &target).is_ok() {
                let _ = std::fs::remove_file(&backup);
            } else {
                let _ = std::fs::rename(&backup, &target);
            }
        }
    }
    let _ = std::fs::remove_file(&artifact);
    if target.is_file() {
        let helper = std::env::current_exe().unwrap_or_default();
        let _ = Command::new(&target)
            .arg("--cleanup-update-helper")
            .arg(helper)
            .spawn();
    }
}

fn cleanup_helper(path: &Path) {
    for _ in 0..20 {
        if std::fs::remove_file(path).is_ok() || !path.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

pub fn handle_update_helper_args() -> bool {
    let args: Vec<OsString> = std::env::args_os().collect();
    if args
        .get(1)
        .is_some_and(|arg| arg.to_str() == Some("--apply-update"))
    {
        apply_update_helper(&args);
        return true;
    }
    if args
        .get(1)
        .is_some_and(|arg| arg.to_str() == Some("--cleanup-update-helper"))
    {
        if let Some(path) = args.get(2).map(PathBuf::from) {
            std::thread::spawn(move || cleanup_helper(&path));
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{checksum_for, newer_than_current, version_parts};

    #[test]
    fn versions_are_strict_three_part_numbers() {
        assert_eq!(version_parts("v0.5.0"), Some([0, 5, 0]));
        assert_eq!(version_parts("1.12.3"), Some([1, 12, 3]));
        assert_eq!(version_parts("0.5"), None);
        assert_eq!(version_parts("0.5.0-beta"), None);
        assert!(!newer_than_current(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn checksum_parser_requires_the_exact_asset() {
        let text = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  KeeperUsagePanel.exe\n";
        assert_eq!(
            checksum_for(text, "KeeperUsagePanel.exe"),
            Some("a".repeat(64))
        );
        assert_eq!(checksum_for(text, "setup.exe"), None);
    }
}
