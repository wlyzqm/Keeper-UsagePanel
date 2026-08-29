use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub endpoint: String,
    pub auth_mode: keeper_core::AuthMode,
    #[serde(skip_serializing)]
    pub password: String,
    pub has_password: bool,
    pub remember_password: bool,
    pub poll_seconds: u32,
    pub display_hold_seconds: u32,
    pub edge_auto_collapse: bool,
    pub fullscreen_auto_hide: bool,
    pub allow_private_http: bool,
    pub allow_invalid_certificates: bool,
    pub auto_start: bool,
    pub theme: String,
    pub accent_color: String,
    pub proxy_url: String,
    pub widget_font: String,
    pub x: Option<i32>,
    pub y: Option<i32>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            auth_mode: keeper_core::AuthMode::Admin,
            password: String::new(),
            has_password: false,
            remember_password: true,
            poll_seconds: 2,
            display_hold_seconds: 16,
            edge_auto_collapse: true,
            fullscreen_auto_hide: true,
            allow_private_http: false,
            allow_invalid_certificates: false,
            auto_start: false,
            theme: "light".into(),
            accent_color: String::new(),
            proxy_url: String::new(),
            widget_font: "HarmonyOS Sans SC".into(),
            x: None,
            y: None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_settings_keep_the_default_display_hold() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.display_hold_seconds, 16);
        assert!(settings.edge_auto_collapse);
        assert!(settings.fullscreen_auto_hide);
        assert!(!settings.allow_invalid_certificates);
    }
}
#[cfg(windows)]
mod platform {
    use super::*;
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    use winreg::{enums::*, RegKey, RegValue};
    const PATH: &str = "Software\\KeeperUsagePanel";
    fn protect(bytes: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
        let input = CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let ok = unsafe {
            if encrypt {
                CryptProtectData(
                    &input,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
            } else {
                CryptUnprotectData(
                    &input,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
            }
        };
        if ok == 0 {
            return Err("Windows 凭据加密失败，请重新输入密码".into());
        }
        let result =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
        unsafe {
            LocalFree(output.pbData as _);
        };
        Ok(result)
    }
    #[cfg(test)]
    #[test]
    fn dpapi_round_trip_for_current_windows_user() {
        let original = b"test-only-password-with-utf8-\xe5\xaf\x86";
        let encrypted = protect(original, true).unwrap();
        assert_ne!(encrypted, original);
        assert_eq!(protect(&encrypted, false).unwrap(), original);
    }
    pub fn load() -> Settings {
        let mut s = Settings::default();
        let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(PATH) else {
            return s;
        };
        s.endpoint = k.get_value("Endpoint").unwrap_or_default();
        s.auth_mode = if k.get_value::<String, _>("AuthMode").unwrap_or_default() == "api_key" {
            keeper_core::AuthMode::ApiKey
        } else {
            keeper_core::AuthMode::Admin
        };
        s.poll_seconds = k
            .get_value::<u32, _>("PollSeconds")
            .unwrap_or(2)
            .clamp(1, 60);
        s.display_hold_seconds = k
            .get_value::<u32, _>("DisplayHoldSeconds")
            .unwrap_or(16)
            .min(300);
        s.edge_auto_collapse = k.get_value::<u32, _>("EdgeAutoCollapse").unwrap_or(1) == 1;
        s.fullscreen_auto_hide = k.get_value::<u32, _>("FullscreenAutoHide").unwrap_or(1) == 1;
        s.remember_password = k.get_value::<u32, _>("RememberPassword").unwrap_or(1) == 1;
        s.allow_private_http = k.get_value::<u32, _>("AllowPrivateHttp").unwrap_or(0) == 1;
        s.allow_invalid_certificates = k
            .get_value::<u32, _>("AllowInvalidCertificates")
            .unwrap_or(0)
            == 1;
        s.auto_start = k.get_value::<u32, _>("AutoStart").unwrap_or(0) == 1;
        s.theme = k.get_value("Theme").unwrap_or("light".into());
        s.accent_color = k.get_value("AccentColor").unwrap_or_default();
        s.x = k.get_value::<u32, _>("X").ok().map(|v| v as i32);
        s.y = k.get_value::<u32, _>("Y").ok().map(|v| v as i32);
        if s.remember_password {
            if let Ok(raw) = k.get_raw_value("ProtectedPassword") {
                if let Ok(bytes) = protect(&raw.bytes, false) {
                    s.password = String::from_utf8(bytes).unwrap_or_default();
                }
            }
        }
        s.widget_font = k
            .get_value("WidgetFont")
            .unwrap_or("HarmonyOS Sans SC".into());
        if let Ok(raw) = k.get_raw_value("ProtectedProxyUrl") {
            // Never silently switch a configured proxy to direct on decryption failure.
            s.proxy_url = protect(&raw.bytes, false)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .unwrap_or("代理无法解密，请重新填写或清空以确认直连".into());
        }
        s.has_password = !s.password.is_empty();
        s
    }
    pub fn save(s: &Settings) -> Result<(), String> {
        let encrypted = if s.remember_password && !s.password.is_empty() {
            Some(protect(s.password.as_bytes(), true)?)
        } else {
            None
        };
        let proxy = if s.proxy_url.is_empty() {
            None
        } else {
            Some(protect(s.proxy_url.as_bytes(), true)?)
        };
        let write = || -> std::io::Result<()> {
            let (k, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(PATH)?;
            k.set_value("Endpoint", &s.endpoint)?;
            k.set_value(
                "AuthMode",
                &if s.auth_mode == keeper_core::AuthMode::ApiKey {
                    "api_key"
                } else {
                    "admin"
                },
            )?;
            k.set_value("PollSeconds", &s.poll_seconds)?;
            k.set_value("DisplayHoldSeconds", &s.display_hold_seconds)?;
            k.set_value("EdgeAutoCollapse", &(s.edge_auto_collapse as u32))?;
            k.set_value("FullscreenAutoHide", &(s.fullscreen_auto_hide as u32))?;
            k.set_value("Theme", &s.theme)?;
            k.set_value("AccentColor", &s.accent_color)?;
            k.set_value("WidgetFont", &s.widget_font)?;
            if let Some(bytes) = proxy {
                k.set_raw_value(
                    "ProtectedProxyUrl",
                    &RegValue {
                        bytes,
                        vtype: REG_BINARY,
                    },
                )?
            } else {
                let _ = k.delete_value("ProtectedProxyUrl");
            }
            k.set_value("RememberPassword", &(s.remember_password as u32))?;
            k.set_value("AllowPrivateHttp", &(s.allow_private_http as u32))?;
            k.set_value(
                "AllowInvalidCertificates",
                &(s.allow_invalid_certificates as u32),
            )?;
            k.set_value("AutoStart", &(s.auto_start as u32))?;
            if let Some(v) = s.x {
                k.set_value("X", &(v as u32))?
            }
            if let Some(v) = s.y {
                k.set_value("Y", &(v as u32))?
            }
            if let Some(bytes) = encrypted {
                k.set_raw_value(
                    "ProtectedPassword",
                    &RegValue {
                        bytes,
                        vtype: REG_BINARY,
                    },
                )?
            } else {
                let _ = k.delete_value("ProtectedPassword");
            }
            let (run, _) = RegKey::predef(HKEY_CURRENT_USER)
                .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")?;
            if s.auto_start {
                run.set_value(
                    "KeeperUsagePanel",
                    &format!("\"{}\"", std::env::current_exe()?.display()),
                )?
            } else {
                let _ = run.delete_value("KeeperUsagePanel");
            }
            Ok(())
        };
        write().map_err(|_| "无法保存当前用户注册表配置".into())
    }
    pub fn position(x: i32, y: i32) {
        if let Ok((k, _)) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(PATH) {
            let _ = k.set_value("X", &(x as u32));
            let _ = k.set_value("Y", &(y as u32));
        }
    }
}
#[cfg(windows)]
pub use platform::*;
#[cfg(not(windows))]
pub fn load() -> Settings {
    Settings::default()
}
#[cfg(not(windows))]
pub fn save(_: &Settings) -> Result<(), String> {
    Err("注册表配置仅支持 Windows".into())
}
#[cfg(not(windows))]
pub fn position(_: i32, _: i32) {}
