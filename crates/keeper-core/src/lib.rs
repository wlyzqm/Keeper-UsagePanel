pub mod metrics;
use chrono::{DateTime, Datelike, Days, Duration, NaiveDate, TimeZone, Utc};
use metrics::*;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
use tokio::sync::Mutex;

#[derive(Default, Clone, Deserialize, Serialize)]
pub struct Query {
    #[serde(default = "today")]
    pub range: String,
    #[serde(default)]
    pub api_key_id: String,
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub end: String,
    #[serde(default)]
    pub account_id: String,
    #[serde(default)]
    pub cursor: String,
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub window_role: String,
}
fn today() -> String {
    "today".into()
}
impl Query {
    pub fn params(&self, now: DateTime<Utc>) -> Result<Vec<(String, String)>, String> {
        let mut p = vec![];
        let range = if self.range.is_empty() {
            "today"
        } else {
            &self.range
        };
        match range {
            "today" | "yesterday" | "7d" | "30d" => p.push(("range".into(), range.into())),
            "month" => {
                let date = now.with_timezone(&beijing()).date_naive();
                p.extend([
                    ("range".into(), "custom".into()),
                    ("unit".into(), "day".into()),
                    (
                        "start".into(),
                        format!("{}-{:02}-01", date.year(), date.month()),
                    ),
                    ("end".into(), date.to_string()),
                ]);
            }
            "custom" => {
                let (a, b) = self.dates()?;
                p.extend([
                    ("range".into(), "custom".into()),
                    ("unit".into(), "day".into()),
                    ("start".into(), a.to_string()),
                    ("end".into(), b.to_string()),
                ]);
            }
            _ => return Err("不支持的日期范围".into()),
        }
        if !self.api_key_id.is_empty() {
            p.push(("api_key_id".into(), self.api_key_id.clone()));
        }
        Ok(p)
    }
    fn dates(&self) -> Result<(NaiveDate, NaiveDate), String> {
        let a = NaiveDate::parse_from_str(&self.start, "%Y-%m-%d").map_err(|_| "请选择开始日期")?;
        let b = NaiveDate::parse_from_str(&self.end, "%Y-%m-%d").map_err(|_| "请选择结束日期")?;
        if a > b {
            return Err("开始日期不能晚于结束日期".into());
        }
        Ok((a, b))
    }
    fn bounds(&self, now: DateTime<Utc>) -> Result<(DateTime<Utc>, DateTime<Utc>), String> {
        let date = now.with_timezone(&beijing()).date_naive();
        let midnight = |d: NaiveDate| {
            beijing()
                .from_local_datetime(&d.and_hms_opt(0, 0, 0).unwrap())
                .unwrap()
                .with_timezone(&Utc)
        };
        let start = midnight(date);
        let end = start + Duration::days(1);
        Ok(match self.range.as_str() {
            "7d" => (now - Duration::days(7), now),
            "30d" => (now - Duration::days(30), now),
            "yesterday" => (start - Duration::days(1), start),
            "month" => (midnight(date.with_day(1).unwrap()), end),
            "custom" => {
                let (a, b) = self.dates()?;
                (midnight(a), midnight(b) + Duration::days(1))
            }
            _ => (start, end),
        })
    }
}
pub fn validate_endpoint(endpoint: &str, allow_http: bool) -> Result<Url, String> {
    let mut url = Url::parse(endpoint.trim()).map_err(|_| "请输入完整的 Keeper 地址")?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("地址只支持 HTTP / HTTPS".into());
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("地址不能包含账号、查询参数或 # 片段".into());
    }
    if url.scheme() == "http"
        && !matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
        && !allow_http
    {
        return Err("公网请使用 HTTPS；专网 HTTP 请勾选允许选项".into());
    }
    url.set_path(&format!("{}/", url.path().trim_end_matches('/')));
    Ok(url)
}
pub struct Keeper {
    http: Client,
    base: Url,
    password: String,
    authenticated: AtomicBool,
    login_gate: Mutex<()>,
    cache: Mutex<HashMap<String, (Instant, Value)>>,
    identities: Mutex<HashMap<String, Value>>,
    counter: Mutex<Counter>,
}
impl Keeper {
    pub fn new(endpoint: &str, password: &str, allow_http: bool) -> Result<Self, String> {
        Self::with_proxy(endpoint, password, allow_http, "")
    }
    pub fn with_proxy(
        endpoint: &str,
        password: &str,
        allow_http: bool,
        proxy_url: &str,
    ) -> Result<Self, String> {
        let base = validate_endpoint(endpoint, allow_http)?;
        let mut builder = Client::builder()
            .no_proxy()
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(25));
        if !proxy_url.trim().is_empty() {
            let proxy = Url::parse(proxy_url.trim())
                .map_err(|_| "代理地址无效，请包含 http:// 或 socks5://")?;
            if !matches!(proxy.scheme(), "http" | "https" | "socks5" | "socks5h")
                || proxy.host_str().is_none()
                || proxy.query().is_some()
                || proxy.fragment().is_some()
                || proxy.path() != "/" && !proxy.path().is_empty()
            {
                return Err(
                    "代理仅支持 HTTP / HTTPS / SOCKS5 / SOCKS5H 地址，不含路径或参数".into(),
                );
            }
            builder = builder
                .proxy(reqwest::Proxy::all(proxy).map_err(|_| "无法创建代理，请检查地址与端口")?);
        }
        let http = builder.build().map_err(|_| "无法创建连接")?;
        Ok(Self {
            http,
            base,
            password: password.into(),
            authenticated: AtomicBool::new(false),
            login_gate: Mutex::new(()),
            cache: Mutex::new(HashMap::new()),
            identities: Mutex::new(HashMap::new()),
            counter: Mutex::new(Counter::default()),
        })
    }
    fn url(&self, path: &str, params: &[(String, String)]) -> Result<Url, String> {
        let mut url = self
            .base
            .join(&format!("api/v1/{path}"))
            .map_err(|_| "接口地址无效")?;
        if !params.is_empty() {
            url.query_pairs_mut().extend_pairs(params);
        }
        Ok(url)
    }
    pub async fn login(&self) -> Result<(), String> {
        let _gate = self.login_gate.lock().await;
        if self.authenticated.load(Ordering::SeqCst) {
            return Ok(());
        }
        let response = self
            .http
            .post(self.url("auth/login", &[])?)
            .header("X-CPA-Usage-Keeper-Request", "fetch")
            .json(&json!({"password":self.password}))
            .send()
            .await
            .map_err(|_| "无法连接 Keeper，请检查地址与网络")?;
        match response.status().as_u16() {
            200 | 204 => {
                self.authenticated.store(true, Ordering::SeqCst);
                Ok(())
            }
            401 | 403 => Err("Keeper 登录密码无效".into()),
            _ => Err("Keeper 登录失败，请检查地址是否包含 /usage".into()),
        }
    }
    pub async fn logout(&self) -> Result<(), String> {
        self.request("auth/logout", &[], Some(json!({}))).await?;
        self.authenticated.store(false, Ordering::SeqCst);
        Ok(())
    }
    async fn request(
        &self,
        path: &str,
        params: &[(String, String)],
        body: Option<Value>,
    ) -> Result<Value, String> {
        for attempt in 0..2 {
            self.login().await?;
            let url = self.url(path, params)?;
            let request = if let Some(ref b) = body {
                self.http
                    .post(url)
                    .header("X-CPA-Usage-Keeper-Request", "fetch")
                    .json(b)
            } else {
                self.http.get(url)
            };
            let response = request
                .send()
                .await
                .map_err(|_| "连接超时或网络不可达，请检查 Keeper 地址")?;
            if response.status().as_u16() == 401 {
                self.authenticated.store(false, Ordering::SeqCst);
                if attempt == 0 {
                    continue;
                }
                return Err("Keeper 会话失效，请重新登录".into());
            }
            if !response.status().is_success() {
                return Err(match response.status().as_u16() {
                    404 => "接口不存在，请检查 /usage 路径与 Keeper 版本",
                    400 => "Keeper 不支持此日期范围或账户类型",
                    _ => "Keeper 暂不可用，请稍后重试",
                }
                .into());
            }
            if response.status().as_u16() == 204 {
                return Ok(json!({}));
            }
            return response
                .json()
                .await
                .map_err(|_| "Keeper 未返回 JSON，请检查地址是否正确".into());
        }
        unreachable!()
    }
    async fn cached(
        &self,
        path: &str,
        params: &[(String, String)],
        ttl: u64,
    ) -> Result<Value, String> {
        let key = self.url(path, params)?.to_string();
        if let Some((at, v)) = self.cache.lock().await.get(&key) {
            if at.elapsed().as_secs() < ttl {
                return Ok(v.clone());
            }
        }
        let value = self.request(path, params, None).await?;
        let mut cache = self.cache.lock().await;
        if cache.len() > 32 {
            cache.clear();
        }
        cache.insert(key, (Instant::now(), value.clone()));
        Ok(value)
    }
    pub async fn sample(&self) -> Result<Value, String> {
        let mut counter = self.counter.lock().await;
        let today = self
            .request("usage/activity", &[("window".into(), "today".into())], None)
            .await?;
        if s(&today, "timezone") != "Asia/Shanghai" {
            return Err("Keeper 时区需设为 Asia/Shanghai，才能按北京时间统计今日用量".into());
        }
        let date = DateTime::parse_from_rfc3339(s(&today, "window_start"))
            .map_err(|_| "Keeper 今日时间字段异常")?
            .with_timezone(&beijing())
            .date_naive();
        if !today["input_tokens"].is_i64()
            || !today["output_tokens"].is_i64()
            || !today["total_tokens"].is_i64()
        {
            return Err("今日累计字段缺失，保留上次采样基线".into());
        }
        let at = Utc::now();
        let reading = Reading {
            day: date,
            input: n(&today, "input_tokens"),
            output: n(&today, "output_tokens"),
            at,
        };
        let mut closed = None;
        if let Some(previous) = &counter.previous {
            if date > previous.day && (date - previous.day).num_days() <= 364 {
                let q = Query {
                    range: "custom".into(),
                    start: previous.day.to_string(),
                    end: date.checked_sub_days(Days::new(1)).unwrap().to_string(),
                    ..Default::default()
                };
                closed = Some(totals(
                    &self.request("usage/analysis", &q.params(at)?, None).await?,
                ));
            }
        }
        let (mut success, mut failure, mut page, mut pages) = (0, 0, 1, 1);
        while page <= pages {
            let data = self
                .cached(
                    "usage/identities/page",
                    &[
                        ("page_size".into(), "100".into()),
                        ("page".into(), page.to_string()),
                        ("active_only".into(), "false".into()),
                    ],
                    10,
                )
                .await?;
            pages = n(&data, "total_pages").max(1);
            for account in rows(&data, "identities") {
                success += n(&account["credential_health"], "total_success");
                failure += n(&account["credential_health"], "total_failure");
            }
            page += 1;
        }
        let delta = counter.accept(reading, closed);
        Ok(
            json!({"sampled_at":at,"timezone":"Asia/Shanghai","today_tokens":n(&today,"total_tokens"),"delta":delta,"health":{"label":health(success,failure),"success":success,"failure":failure}}),
        )
    }
    pub async fn view(&self, view: &str, q: Query) -> Result<Value, String> {
        let now = Utc::now();
        let params = q.params(now)?;
        match view {
            "keys" => return self.cached("usage/api-keys/options", &[], 60).await,
            "summary" => {
                let overview = self.cached("usage/overview", &params, 8).await?;
                let analysis = self.cached("usage/analysis", &params, 15).await?;
                return Ok(json!({"overview":overview,"activity":totals(&analysis)}));
            }
            "analysis" => return self.cached("usage/analysis", &params, 15).await,
            "latency" => return self.cached("usage/analysis/latency", &params, 30).await,
            "accounts" => {
                let data = self
                    .cached(
                        "usage/identities/page",
                        &[
                            ("auth_type".into(), "1".into()),
                            ("page_size".into(), "20".into()),
                            ("page".into(), q.page.max(1).to_string()),
                        ],
                        30,
                    )
                    .await?;
                let mut identities = self.identities.lock().await;
                for a in rows(&data, "identities") {
                    identities.insert(
                        a["id"]
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| a["id"].to_string()),
                        a.clone(),
                    );
                }
                return Ok(data);
            }
            _ => {}
        }
        let identity = self
            .identities
            .lock()
            .await
            .get(&q.account_id)
            .cloned()
            .ok_or("请重新选择认证账户")?;
        let auth = s(&identity, "identity");
        let escape = |value: &str| {
            let mut u = Url::parse("https://placeholder.invalid/").unwrap();
            u.path_segments_mut().unwrap().push(value);
            u.path().trim_start_matches('/').to_string()
        };
        match view {
            "quota" => {
                self.request("quota/cache", &[], Some(json!({"auth_indexes":[auth]})))
                    .await
            }
            "quota-history" => {
                if !s(&identity, "type").eq_ignore_ascii_case("codex") {
                    return Ok(json!({"supported":false,"cycles":[]}));
                }
                let role = if q.window_role == "secondary" {
                    "secondary"
                } else {
                    "primary"
                };
                self.cached(
                    &format!("quota/history/{}", escape(auth)),
                    &[("window_role".into(), role.into())],
                    30,
                )
                .await
            }
            "requests" => {
                let mut p = params;
                p.extend([
                    ("auth_type".into(), "1".into()),
                    ("source".into(), auth.into()),
                    ("page_size".into(), "50".into()),
                    ("cursor_mode".into(), "true".into()),
                ]);
                if !q.cursor.is_empty() {
                    p.push(("cursor".into(), q.cursor))
                }
                self.request("usage/events", &p, None).await
            }
            "errors" => {
                let mut p = vec![("page_size".into(), "50".into())];
                if !q.cursor.is_empty() {
                    p.push(("cursor".into(), q.cursor.clone()))
                }
                let data = self
                    .request(
                        &format!("usage/identities/{}/errors", escape(&q.account_id)),
                        &p,
                        None,
                    )
                    .await?;
                filter_errors(data, &q, now)
            }
            _ => Err("未知视图".into()),
        }
    }
}
pub fn filter_errors(data: Value, q: &Query, now: DateTime<Utc>) -> Result<Value, String> {
    let (from, to) = q.bounds(now)?;
    let mut events = vec![];
    let mut oldest = None;
    for event in rows(&data, "events") {
        if let Ok(t) = DateTime::parse_from_rfc3339(s(event, "timestamp")) {
            let t = t.with_timezone(&Utc);
            oldest = Some(oldest.map_or(t, |o: DateTime<Utc>| o.min(t)));
            if t >= from && t < to {
                events.push(event.clone())
            }
        }
    }
    Ok(
        json!({"total_count":events.len(),"events":events,"has_more":data["has_more"]==true&&oldest.is_none_or(|o|o>=from),"next_cursor":data["next_cursor"],"scope_notice":"仅统计本页日期范围内的事件，不代表范围总数；错误不支持按 Key 归属。"}),
    )
}
