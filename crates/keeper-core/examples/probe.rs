//! Read-only live verification. Credentials arrive via stdin, never CLI arguments or logs.
use keeper_core::{Keeper, Query};
use serde_json::Value;
use std::io::Read;

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|_| "无法读取连接配置")?;
    let config: Value = serde_json::from_str(&input).map_err(|_| "连接配置无效")?;
    let client = Keeper::with_proxy(
        config["endpoint"].as_str().ok_or("缺少地址")?,
        config["password"].as_str().unwrap_or(""),
        true,
        config["proxy"].as_str().unwrap_or(""),
    )?;
    let result = async {
        let first = client.sample().await?;
        println!("sample: OK; timezone={}", first["timezone"]);
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let next = client.sample().await?;
        println!(
            "interval: input={} output={} seconds={}",
            next["delta"]["input_tokens"], next["delta"]["output_tokens"], next["delta"]["seconds"]
        );
        for view in ["keys", "summary", "analysis", "latency"] {
            client
                .view(
                    view,
                    Query {
                        range: "today".into(),
                        ..Default::default()
                    },
                )
                .await?;
            println!("{view}: OK");
        }
        let accounts = client.view("accounts", Query::default()).await?;
        if let Some(account) = accounts["identities"].as_array().and_then(|a| a.first()) {
            let id = account["id"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| account["id"].to_string());
            for view in ["quota", "quota-history", "requests", "errors"] {
                client
                    .view(
                        view,
                        Query {
                            range: "today".into(),
                            account_id: id.clone(),
                            ..Default::default()
                        },
                    )
                    .await?;
                println!("account {view}: OK");
            }
        }
        Ok::<(), String>(())
    }
    .await;
    let logout = client.logout().await;
    println!("logout: {}", if logout.is_ok() { "OK" } else { "FAILED" });
    result.and(logout)
}
