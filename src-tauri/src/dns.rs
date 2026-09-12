use serde::Deserialize;
use shared::DnsTestResult;
use std::time::Instant;

#[derive(Deserialize)]
struct DohAnswer {
    data: String,
}

#[derive(Deserialize)]
struct DohPayload {
    #[serde(rename = "Answer")]
    answer: Option<Vec<DohAnswer>>,
}

pub struct DnsResolver;

impl DnsResolver {
    pub async fn ping_test(doh_url: &str) -> DnsTestResult {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(4))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return DnsTestResult {
                    latency_ms: 0,
                    resolved_ip: None,
                    success: false,
                    error: Some(e.to_string()),
                }
            }
        };

        let separator = if doh_url.contains('?') { "&" } else { "?" };
        let test_endpoint = format!("{}{}name=cloudflare.com&type=A", doh_url, separator);

        let start = Instant::now();
        let request = client
            .get(&test_endpoint)
            .header("accept", "application/dns-json")
            .send()
            .await;

        let elapsed = start.elapsed().as_millis();

        match request {
            Ok(res) => {
                if !res.status().is_success() {
                    return DnsTestResult {
                        latency_ms: elapsed,
                        resolved_ip: None,
                        success: false,
                        error: Some(format!("HTTP {}", res.status())),
                    };
                }
                match res.json::<DohPayload>().await {
                    Ok(payload) => {
                        let resolved = payload
                            .answer
                            .and_then(|a| a.into_iter().next())
                            .map(|ans| ans.data);
                        DnsTestResult {
                            latency_ms: elapsed,
                            resolved_ip: resolved,
                            success: true,
                            error: None,
                        }
                    }
                    Err(e) => DnsTestResult {
                        latency_ms: elapsed,
                        resolved_ip: None,
                        success: false,
                        error: Some(format!("Payload Parse Error: {}", e)),
                    },
                }
            }
            Err(e) => DnsTestResult {
                latency_ms: elapsed,
                resolved_ip: None,
                success: false,
                error: Some(e.to_string()),
            },
        }
    }
}
