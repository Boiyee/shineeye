use std::{
    sync::LazyLock,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::Lang;

static SOLANA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[1-9A-HJ-NP-Za-km-z]{32,44}\b").unwrap());
static EVM_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b0x[a-fA-F0-9]{40}\b").unwrap());

#[derive(Debug, Clone, Serialize)]
pub struct WalletFinding {
    pub chain: &'static str,
    pub address: String,
    pub checked: bool,
    pub tx_count: Option<usize>,
    pub age_days: Option<u64>,
    pub flags: Vec<String>,
    pub weight: u8,
}

pub fn find_address(text: &str) -> Option<(&'static str, &str)> {
    EVM_RE.find(text).map(|m| ("evm", m.as_str())).or_else(|| {
        // base58 strings must mix digits and letters to avoid matching long words
        SOLANA_RE
            .find_iter(text)
            .map(|m| m.as_str())
            .find(|s| {
                s.chars().any(|c| c.is_ascii_digit()) && s.chars().any(|c| c.is_ascii_alphabetic())
            })
            .map(|s| ("solana", s))
    })
}

#[derive(Deserialize)]
struct RpcResponse {
    result: Vec<Signature>,
}

#[derive(Deserialize)]
struct Signature {
    #[serde(rename = "blockTime")]
    block_time: Option<u64>,
}

pub async fn check(
    http: &reqwest::Client,
    rpc: &str,
    text: &str,
    lang: Lang,
) -> Option<WalletFinding> {
    let (chain, address) = find_address(text)?;
    let mut f = WalletFinding {
        chain,
        address: address.to_owned(),
        checked: false,
        tx_count: None,
        age_days: None,
        flags: vec![lang
            .pick(
                "Asks you to send crypto to a wallet address",
                "Dem wan make you send crypto enter wallet",
            )
            .to_owned()],
        weight: 15,
    };
    if chain != "solana" {
        return Some(f);
    }

    let body = json!({
        "jsonrpc": "2.0", "id": 1, "method": "getSignaturesForAddress",
        "params": [address, { "limit": 1000 }]
    });
    let req = http.post(rpc).json(&body).timeout(Duration::from_secs(4));
    let Ok(Ok(resp)) = tokio::time::timeout(Duration::from_secs(5), async {
        req.send()
            .await?
            .error_for_status()?
            .json::<RpcResponse>()
            .await
    })
    .await
    else {
        return Some(f);
    };

    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let oldest = resp.result.iter().filter_map(|s| s.block_time).min();
    f.checked = true;
    f.tx_count = Some(resp.result.len());
    f.age_days = oldest.map(|t| now.saturating_sub(t) / 86_400);

    match (f.tx_count, f.age_days) {
        (Some(0), _) => {}
        (Some(n), Some(d)) if d < 14 => {
            f.flags.push(match lang {
                Lang::En => format!("Wallet is only {d} days old but already has {n} transactions"),
                Lang::Pcm => format!("Dis wallet na {d} days old, but e don get {n} transactions"),
            });
            f.weight += 25;
        }
        (Some(n), Some(d)) if d < 60 => {
            f.flags.push(match lang {
                Lang::En => format!("Young wallet: first used {d} days ago, {n} transactions"),
                Lang::Pcm => format!("New wallet: dem start am {d} days ago, {n} transactions"),
            });
            f.weight += 10;
        }
        _ => {}
    }
    Some(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_solana_address() {
        let t = "Send 50 USDT to 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU and get 100";
        assert_eq!(find_address(t).map(|a| a.0), Some("solana"));
    }

    #[test]
    fn finds_evm_address() {
        let t = "deposit to 0x52908400098527886E0F7030069857D2E4169EE7 today";
        assert_eq!(find_address(t).map(|a| a.0), Some("evm"));
    }

    #[test]
    fn ignores_plain_text() {
        assert!(find_address("Your account has been credited with NGN 50,000").is_none());
    }
}
