use keeper_core::{filter_errors, validate_endpoint, AuthMode, Keeper, Query};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Mutex,
};

async fn server(
    responses: Vec<(u16, Value)>,
) -> (String, Arc<Mutex<Vec<String>>>, tokio::task::JoinHandle<()>) {
    server_as(responses, "admin").await
}
async fn server_as(
    responses: Vec<(u16, Value)>,
    role: &str,
) -> (String, Arc<Mutex<Vec<String>>>, tokio::task::JoinHandle<()>) {
    let responses: Vec<_> = responses.into_iter().flat_map(|response| {
        if response.0 == 204 { vec![response, (200, json!({"authenticated":true,"role":role,"api_key":{"alias":"Example","display_key":"sk-***123456"}}))] }
        else { vec![response] }
    }).collect();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let captured = Arc::new(Mutex::new(vec![]));
    let out = captured.clone();
    let task = tokio::spawn(async move {
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![];
            loop {
                let mut buf = [0; 4096];
                let n = stream.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..n]);
                if let Some(i) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let h = String::from_utf8_lossy(&request[..i]);
                    let length = h
                        .lines()
                        .find_map(|line| {
                            line.to_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|s| s.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if request.len() >= i + 4 + length {
                        break;
                    }
                }
            }
            out.lock().await.push(String::from_utf8(request).unwrap());
            let text = if status == 204 {
                String::new()
            } else {
                body.to_string()
            };
            let cookie = if status == 204 {
                "Set-Cookie: keeper_session=test-only; Path=/usage/; HttpOnly\r\n"
            } else {
                ""
            };
            stream.write_all(format!("HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{cookie}Connection: close\r\n\r\n{text}",text.len()).as_bytes()).await.unwrap();
        }
    });
    (format!("http://{addr}/usage"), captured, task)
}
fn activity(input: i64, output: i64) -> Value {
    json!({"timezone":"Asia/Shanghai","window_start":"2026-08-26T00:00:00+08:00","input_tokens":input,"output_tokens":output,"total_tokens":input+output})
}
fn request_health(success: i64, failure: i64) -> Value {
    let window_end = chrono::Utc::now() + chrono::Duration::minutes(1);
    json!({
        "window_end":window_end,
        "total_success":success,
        "total_failure":failure+85,
        "blocks":[
            {
                "start_time":window_end-chrono::Duration::hours(5)-chrono::Duration::seconds(1),
                "success":0,
                "failure":85
            },
            {
                "start_time":window_end-chrono::Duration::minutes(5),
                "success":success,
                "failure":failure
            }
        ]
    })
}

#[tokio::test]
async fn direct_login_cookie_and_failed_poll_preserves_baseline() {
    let(url,requests,task)=server(vec![(204,json!(null)),(200,activity(100,30)),(200,json!({"total_pages":1,"identities":[{"credential_health":{"total_success":9,"total_failure":1}}]})),(500,json!({})),(200,activity(125,42))]).await;
    let client = Keeper::new(&url, "fixture-password", false).unwrap();
    let first = client.sample().await.unwrap();
    assert_eq!(first["delta"]["baseline"], true);
    assert!(client.sample().await.is_err());
    let last = client.sample().await.unwrap();
    assert_eq!(last["delta"]["input_tokens"], 25);
    assert_eq!(last["delta"]["output_tokens"], 12);
    assert_eq!(last["health"]["label"], "健康");
    task.await.unwrap();
    let requests = requests.lock().await;
    assert!(requests[0].starts_with("POST /usage/api/v1/auth/login "));
    assert!(requests[0]
        .to_lowercase()
        .contains("x-cpa-usage-keeper-request: fetch"));
    assert!(requests[1]
        .to_lowercase()
        .contains("cookie: keeper_session=test-only"));
    assert!(requests[3].contains("active_only=false"));
}
#[tokio::test]
async fn exact_dates_key_filters_and_no_adapter() {
    let(url,requests,task)=server(vec![(204,json!(null)),(200,json!({"options":[{"id":"abc","label":"Example"}]})),(200,json!({"usage":{"total_requests":3}})),(200,json!({"token_usage":[{"input_tokens":10,"output_tokens":3,"total_tokens":13},{"input_tokens":20,"output_tokens":4,"total_tokens":24}]}))]).await;
    let client = Keeper::new(&url, "", false).unwrap();
    let keys = client.view("keys", Query::default()).await.unwrap();
    assert_eq!(keys["options"][0]["id"], "abc");
    let data = client
        .view(
            "summary",
            Query {
                range: "custom".into(),
                start: "2026-08-01".into(),
                end: "2026-08-26".into(),
                api_key_id: "key & value".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(data["activity"]["output_tokens"], 7);
    assert_eq!(data["activity"]["total_tokens"], 37);
    task.await.unwrap();
    let requests = requests.lock().await;
    assert!(requests[4].starts_with("GET /usage/api/v1/usage/analysis?"));
    assert!(requests[4].contains("api_key_id=key+%26+value"));
    assert!(requests[4].contains("start=2026-08-01&end=2026-08-26"));
    assert!(!requests
        .iter()
        .any(|r| r.contains("usage/activity") || r.contains("quota/refresh")));
}
#[tokio::test]
async fn readonly_account_quota_and_request_routes() {
    let (url, requests, task) = server(vec![
        (204, json!(null)),
        (
            200,
            json!({"total_pages":2,"total_count":2,"identities":[{"id":"a1","identity":"auth/index","type":"codex"}]}),
        ),
        (200, json!({"total_pages":2,"total_count":2,"identities":[{"id":"a2","identity":"other/index","type":"codex"}]})),
        (200, json!({"items":[]})),
        (200, json!({"items":[]})),
        (200, json!({"cycles":[]})),
        (200, json!({"events":[]})),
    ])
    .await;
    let client = Keeper::new(&url, "", false).unwrap();
    let accounts = client.view("accounts", Query::default()).await.unwrap();
    assert_eq!(accounts["identities"].as_array().unwrap().len(), 2);
    let q = Query {
        account_id: "a1".into(),
        range: "today".into(),

        ..Default::default()
    };
    client.view("quota", q.clone()).await.unwrap();
    client.view("quota-history", q.clone()).await.unwrap();
    client.view("requests", q).await.unwrap();
    task.await.unwrap();
    let requests = requests.lock().await;
    assert!(requests[2].contains("page_size=100&page=1"));
    assert!(requests[3].contains("page_size=100&page=2"));
    assert!(requests[4].starts_with("POST /usage/api/v1/quota/cache "));
    assert!(requests[4].contains("auth/index"));
    assert!(requests[4].contains("other/index"));
    assert!(requests[5].starts_with("POST /usage/api/v1/quota/cache "));
    assert!(requests[6].contains("quota/history/auth%2Findex"));
    assert!(requests[7].contains("source=auth%2Findex"));
    assert!(requests.iter().all(|request| request
        .to_lowercase()
        .contains("user-agent: keeper-usagepanel_0.5.4 admin")));
}

#[tokio::test]
async fn summary_reports_partial_and_missing_cost_coverage() {
    for (models, priced, unpriced) in [
        (
            json!([
                {"model":"priced","requests":1,"total_tokens":10,"cost_available":true},
                {"model":"unpriced","requests":1,"total_tokens":20,"cost_available":false}
            ]),
            1,
            1,
        ),
        (
            json!([
                {"model":"unpriced-a","requests":1,"total_tokens":10,"cost_available":false},
                {"model":"unpriced-b","requests":1,"total_tokens":20,"cost_available":false}
            ]),
            0,
            2,
        ),
    ] {
        let (url, _, task) = server(vec![
            (204, json!(null)),
            (200, json!({"usage":{"total_tokens":30},"summary":{"total_cost":1.25,"cost_available":false}})),
            (200, json!({"cost_breakdown":{"total_cost_usd":1.25,"cost_available":false},"model_efficiency":models})),
        ])
        .await;
        let client = Keeper::new(&url, "", false).unwrap();
        let summary = client.view("summary", Query::default()).await.unwrap();
        assert_eq!(summary["cost_coverage"]["priced_models"], priced);
        assert_eq!(
            summary["cost_coverage"]["unpriced_models"]
                .as_array()
                .unwrap()
                .len(),
            unpriced
        );
        assert_eq!(summary["cost_coverage"]["complete"], false);
        task.await.unwrap();
    }
}
#[test]
fn error_pages_report_only_matching_page_count() {
    let now = chrono::DateTime::parse_from_rfc3339("2026-08-26T12:00:00+08:00")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let result=filter_errors(json!({"events":[{"timestamp":"2026-08-26T02:00:00+08:00"},{"timestamp":"2026-08-25T23:59:59+08:00"}],"has_more":true,"total_count":600}),&Query{range:"today".into(),..Default::default()},now).unwrap();
    assert_eq!(result["total_count"], 1);
    assert_eq!(result["has_more"], false);
}
#[test]
fn address_and_date_validation() {
    assert!(validate_endpoint("http://public.example/usage", false).is_err());
    assert!(validate_endpoint("http://public.example/usage", true).is_ok());
    assert!(validate_endpoint("https://user:secret@example.com", true).is_err());
    assert!(validate_endpoint("file:///secret", true).is_err());
    assert!(Query {
        range: "custom".into(),
        start: "2026-08-27".into(),
        end: "2026-08-26".into(),
        ..Default::default()
    }
    .params(chrono::Utc::now())
    .is_err());
}

#[tokio::test]
async fn http_proxy_is_used_for_keeper_login() {
    let (proxy, requests, task) = server(vec![(204, json!(null))]).await;
    let proxy = proxy.trim_end_matches("/usage");
    let client = Keeper::with_proxy("http://keeper.invalid/usage", "", true, proxy).unwrap();
    client.login().await.unwrap();
    task.await.unwrap();
    assert!(
        requests.lock().await[0].starts_with("POST http://keeper.invalid/usage/api/v1/auth/login ")
    );
}

#[tokio::test]
async fn connection_failure_returns_actionable_errlog_without_proxy_credentials() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let client = Keeper::with_proxy(
        "http://keeper.invalid/usage",
        "",
        true,
        &format!("http://diagnostic-user:diagnostic-secret@{addr}"),
    )
    .unwrap();
    let error = client.login().await.unwrap_err();
    assert!(error.contains("ERRLOG\n"));
    assert!(error.contains("stage=login.request"));
    assert!(error.contains("route=proxy http://127.0.0.1:"));
    assert!(error.contains("tls_verify=enabled"));
    assert!(error.contains("connect=true"));
    assert!(!error.contains("diagnostic-user"));
    assert!(!error.contains("diagnostic-secret"));
}

#[tokio::test]
async fn connection_errlog_reports_when_tls_verification_is_disabled() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let client = Keeper::connect_with_tls(
        "https://keeper.invalid/usage",
        "",
        false,
        &format!("http://{addr}"),
        AuthMode::Admin,
        true,
    )
    .unwrap();
    let error = client.login().await.unwrap_err();
    assert!(error.contains("tls_verify=disabled"));
}

#[tokio::test]
async fn socks5h_proxy_resolves_keeper_on_proxy() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        for session in [false, true] {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut head = [0; 2];
            stream.read_exact(&mut head).await.unwrap();
            assert_eq!(head[0], 5);
            let mut methods = vec![0; head[1] as usize];
            stream.read_exact(&mut methods).await.unwrap();
            assert!(methods.contains(&0));
            stream.write_all(&[5, 0]).await.unwrap();
            let mut command = [0; 5];
            stream.read_exact(&mut command).await.unwrap();
            assert_eq!(&command[..4], &[5, 1, 0, 3]);
            let mut host = vec![0; command[4] as usize];
            stream.read_exact(&mut host).await.unwrap();
            assert_eq!(host, b"keeper.invalid");
            let mut port = [0; 2];
            stream.read_exact(&mut port).await.unwrap();
            assert_eq!(u16::from_be_bytes(port), 80);
            stream
                .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 80])
                .await
                .unwrap();
            let mut request = vec![];
            loop {
                let mut b = [0; 1024];
                let count = stream.read(&mut b).await.unwrap();
                request.extend_from_slice(&b[..count]);
                if count == 0 || request.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let path = if session {
                "GET /usage/api/v1/auth/session "
            } else {
                "POST /usage/api/v1/auth/login "
            };
            assert!(String::from_utf8_lossy(&request).starts_with(path));
            let body = if session {
                r#"{"authenticated":true,"role":"admin"}"#
            } else {
                ""
            };
            let status = if session { 200 } else { 204 };
            stream.write_all(format!("HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        }
    });
    let client = Keeper::with_proxy(
        "http://keeper.invalid/usage",
        "",
        true,
        &format!("socks5h://{addr}"),
    )
    .unwrap();
    client.login().await.unwrap();
    task.await.unwrap();
}
#[test]
fn proxy_schemes_and_paths_are_validated() {
    for proxy in [
        "ftp://127.0.0.1:1080",
        "http://127.0.0.1:1080/path",
        "not a url",
    ] {
        assert!(Keeper::with_proxy("https://keeper.example/usage", "", false, proxy).is_err());
    }
    for proxy in [
        "socks5://127.0.0.1:1080",
        "socks5h://127.0.0.1:1080",
        "http://127.0.0.1:1080",
    ] {
        assert!(Keeper::with_proxy("https://keeper.example/usage", "", false, proxy).is_ok());
    }
}

#[tokio::test]
async fn sk_uses_only_own_endpoints_and_rejects_admin_views() {
    let (url, requests, task) = server_as(
        vec![
            (204, json!(null)),
            (200, activity(100, 30)),
            (200, request_health(99, 1)),
            (200, json!({"usage":{"total_tokens":130}})),
            (200, activity(100, 30)),
        ],
        "api_key_viewer",
    )
    .await;
    let client = Keeper::connect(&url, "sk-fixture-only", false, "", AuthMode::ApiKey).unwrap();
    let sample = client.sample().await.unwrap();
    assert_eq!(sample["today_tokens"], 130);
    assert_eq!(sample["health"]["basis"], "key_requests");
    assert_eq!(sample["health"]["failure"], 1);
    let summary = client.view("summary", Query::default()).await.unwrap();
    assert_eq!(summary["activity"]["output_tokens"], 30);
    for view in [
        "keys",
        "analysis",
        "latency",
        "accounts",
        "quota",
        "quota-history",
        "requests",
        "errors",
    ] {
        assert!(client.view(view, Query::default()).await.is_err(), "{view}");
    }
    assert!(client.sample_scoped("42", 1).await.is_err());
    assert!(client
        .view(
            "summary",
            Query {
                api_key_id: "42".into(),
                ..Default::default()
            }
        )
        .await
        .is_err());
    task.await.unwrap();
    let requests = requests.lock().await;
    assert!(requests[0].starts_with("POST /usage/api/v1/auth/api-key-login "));
    assert!(requests[0].contains(r#""apiKey":"sk-fixture-only""#));
    assert!(requests[0]
        .to_lowercase()
        .contains("user-agent: keeper-usagepanel_0.5.4 sk"));
    assert!(requests[1].contains("/auth/session "));
    assert!(requests[3].contains("key-activity?range=5h"));
    assert!(requests
        .iter()
        .all(|r| !r.contains("api_key_id=") && !r.contains("/api/v1/usage/")));
}
#[tokio::test]
async fn key_switch_rebuilds_delta_and_scopes_health_without_account_data() {
    let (url, requests, task) = server(vec![
        (204, json!(null)),
        (200, activity(100, 30)),
        (200, request_health(9, 1)),
        (200, activity(125, 42)),
        (200, activity(900, 500)),
        (200, request_health(1, 9)),
        (200, activity(130, 45)),
    ])
    .await;
    let client = Keeper::new(&url, "", false).unwrap();
    assert_eq!(
        client.sample_scoped("42", 1).await.unwrap()["delta"]["baseline"],
        true
    );
    assert_eq!(
        client.sample_scoped("42", 1).await.unwrap()["delta"]["input_tokens"],
        25
    );
    let other = client.sample_scoped("43", 2).await.unwrap();
    assert_eq!(other["delta"]["baseline"], true);
    assert_eq!(other["health"]["label"], "异常");
    assert_eq!(
        client.sample_scoped("42", 3).await.unwrap()["delta"]["baseline"],
        true
    );
    assert!(client
        .view(
            "quota",
            Query {
                api_key_id: "42".into(),
                ..Default::default()
            }
        )
        .await
        .is_err());
    task.await.unwrap();
    let requests = requests.lock().await;
    assert!(requests[2..]
        .iter()
        .all(|r| r.contains("api_key_id=4") && !r.contains("identities")));
}
#[tokio::test]
async fn sk_fails_closed_on_admin_session_and_invalid_secret_does_not_loop_login() {
    let (url, requests, task) = server(vec![(204, json!(null))]).await;
    let client = Keeper::connect(&url, "sk-fixture", false, "", AuthMode::ApiKey).unwrap();
    assert!(client.sample().await.is_err());
    assert!(client.sample().await.is_err());
    task.await.unwrap();
    assert_eq!(requests.lock().await.len(), 2);
    let (url, requests, task) = server(vec![(401, json!({}))]).await;
    let client = Keeper::new(&url, "wrong-fixture", false).unwrap();
    assert!(client.login().await.is_err());
    assert!(client.login().await.is_err());
    task.await.unwrap();
    assert_eq!(requests.lock().await.len(), 1);
}

#[tokio::test]
async fn sk_midnight_bridge_uses_own_activity_totals() {
    let mut next_day = activity(8, 2);
    next_day["window_start"] = json!("2026-08-27T00:00:00+08:00");
    let (url, requests, task) = server_as(
        vec![
            (204, json!(null)),
            (200, activity(100, 40)),
            (200, request_health(9, 1)),
            (200, next_day),
            (200, activity(110, 50)),
        ],
        "api_key_viewer",
    )
    .await;
    let client = Keeper::connect(&url, "sk-fixture", false, "", AuthMode::ApiKey).unwrap();
    client.sample().await.unwrap();
    let next = client.sample().await.unwrap();
    assert_eq!(next["delta"]["input_tokens"], 18);
    assert_eq!(next["delta"]["output_tokens"], 12);
    task.await.unwrap();
    let requests = requests.lock().await;
    assert!(
        requests[5].contains("key-activity?range=custom&unit=day&start=2026-08-26&end=2026-08-26")
    );
}

#[test]
fn cpa_console_matches_keeper_public_url_rules_and_rejects_unsafe_links() {
    let base = validate_endpoint("https://keeper.example/usage", false).unwrap();
    for (raw, expected) in [
        (None, "https://keeper.example/management.html"),
        (
            Some("https://cpa.example"),
            "https://cpa.example/management.html",
        ),
        (Some("/cpa/"), "https://keeper.example/cpa/management.html"),
        (
            Some("cpa.example:8317/"),
            "https://cpa.example:8317/management.html",
        ),
        (
            Some("https://cpa.example/cpa/management.html"),
            "https://cpa.example/cpa/management.html",
        ),
    ] {
        assert_eq!(keeper_core::cpa_console_url(&base, raw).unwrap(), expected);
    }
    for raw in [
        "javascript://alert(1)",
        "file:///secret",
        "ftp://cpa.example",
        "https://user:secret@cpa.example",
        "",
    ] {
        assert!(keeper_core::cpa_console_url(&base, Some(raw)).is_err());
    }
}
#[tokio::test]
async fn cpa_console_uses_admin_status_only_and_sk_never_requests_it() {
    let (url, requests, task) = server(vec![
        (204, json!(null)),
        (200, json!({"cpa_public_url":"https://cpa.example/cpa/"})),
    ])
    .await;
    let admin = Keeper::new(&url, "", false).unwrap();
    assert_eq!(
        admin.console_url().await.unwrap(),
        "https://cpa.example/cpa/management.html"
    );
    task.await.unwrap();
    assert!(requests.lock().await[2].starts_with("GET /usage/api/v1/status "));
    let viewer = Keeper::connect(&url, "sk-fixture", false, "", AuthMode::ApiKey).unwrap();
    assert!(viewer.console_url().await.unwrap_err().contains("管理员"));
}
