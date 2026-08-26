use keeper_core::{filter_errors, validate_endpoint, Keeper, Query};
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
    assert!(requests[2].contains("active_only=false"));
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
    assert!(requests[3].starts_with("GET /usage/api/v1/usage/analysis?"));
    assert!(requests[3].contains("api_key_id=key+%26+value"));
    assert!(requests[3].contains("start=2026-08-01&end=2026-08-26"));
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
            json!({"identities":[{"id":"a1","identity":"auth/index","type":"codex"}]}),
        ),
        (200, json!({"items":[]})),
        (200, json!({"cycles":[]})),
        (200, json!({"events":[]})),
    ])
    .await;
    let client = Keeper::new(&url, "", false).unwrap();
    client.view("accounts", Query::default()).await.unwrap();
    let q = Query {
        account_id: "a1".into(),
        range: "today".into(),
        api_key_id: "key1".into(),
        ..Default::default()
    };
    client.view("quota", q.clone()).await.unwrap();
    client.view("quota-history", q.clone()).await.unwrap();
    client.view("requests", q).await.unwrap();
    task.await.unwrap();
    let requests = requests.lock().await;
    assert!(requests[2].starts_with("POST /usage/api/v1/quota/cache "));
    assert!(requests[2].contains("auth/index"));
    assert!(requests[3].contains("quota/history/auth%2Findex"));
    assert!(requests[4].contains("api_key_id=key1"));
    assert!(requests[4].contains("source=auth%2Findex"));
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
async fn socks5h_proxy_resolves_keeper_on_proxy() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
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
        assert!(String::from_utf8_lossy(&request).starts_with("POST /usage/api/v1/auth/login "));
        stream
            .write_all(b"HTTP/1.1 204 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
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
