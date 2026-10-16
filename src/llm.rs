use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{Config, Lang};

const SYSTEM: &str = r#"You are ShineEye, a scam checker for people in Nigeria.
The user pastes something they received: an SMS, WhatsApp or email message, a link, a bank account number or a crypto wallet.
Decide whether it is a scam. You also get findings from deterministic checks (rules, link inspection, wallet lookup); treat them as strong evidence but use judgement.

Know the common Nigerian scams: fake credit alerts and "I sent excess, refund me", OTP/PIN/BVN/NIN requests, "your account will be blocked", fake bank or CBN/NIBSS/EFCC links, Ponzi and "double your money" schemes, crypto doubling, fake jobs or loans with upfront fees, "it's me, new number, send money urgently", fake prizes and giveaways, romance scams.
Genuine bank messages never ask for OTP, PIN, BVN or passwords, and never ask you to reply with codes or click to "verify".

Be calibrated. If the text is ordinary and asks for nothing risky, say "safe". Use "suspicious" when signals are mixed.

Reply with JSON only, exactly this shape:
{"verdict":"scam"|"suspicious"|"safe","confidence":0-100,"scam_type":"short_snake_case_label or none","summary":"one sentence","red_flags":[{"quote":"exact words copied from the input","why":"short reason"}],"advice":["short action","short action"]}

Rules for red_flags: "quote" must be copied character for character from the input; never invent quotes; at most 4 flags; empty list if safe.
Advice: 2 or 3 concrete actions (e.g. do not reply, call your bank with the number on your card, report to your bank). For "safe", advise normal caution.
Write summary, why and advice in LANGUAGE."#;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Scam,
    Suspicious,
    Safe,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Flag {
    pub quote: String,
    pub why: String,
}

#[derive(Debug, Deserialize)]
pub struct ModelVerdict {
    pub verdict: Verdict,
    pub confidence: u8,
    #[serde(default)]
    pub scam_type: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub red_flags: Vec<Flag>,
    #[serde(default)]
    pub advice: Vec<String>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}
#[derive(Deserialize)]
struct Choice {
    message: Msg,
}
#[derive(Deserialize)]
struct Msg {
    content: String,
}

pub async fn judge(
    http: &reqwest::Client,
    cfg: &Config,
    input: &str,
    evidence: &serde_json::Value,
    lang: Lang,
) -> anyhow::Result<ModelVerdict> {
    let language = lang.pick("plain English", "Nigerian Pidgin");
    let system = SYSTEM.replace("LANGUAGE", language);
    let user = format!("INPUT:\n<<<\n{input}\n>>>\n\nCHECK FINDINGS:\n{evidence}");
    let body = json!({
        "model": cfg.llm_model,
        "temperature": 0.1,
        "response_format": { "type": "json_object" },
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ]
    });

    let mut last_err = anyhow::anyhow!("no attempt");
    for _ in 0..2 {
        let mut req = http
            .post(format!(
                "{}/chat/completions",
                cfg.llm_base_url.trim_end_matches('/')
            ))
            .json(&body);
        if let Some(key) = &cfg.llm_api_key {
            req = req.bearer_auth(key);
        }
        let attempt = async {
            let resp: ChatResponse = req.send().await?.error_for_status()?.json().await?;
            let content = &resp
                .choices
                .first()
                .ok_or_else(|| anyhow::anyhow!("empty choices"))?
                .message
                .content;
            Ok::<_, anyhow::Error>(serde_json::from_str::<ModelVerdict>(strip_fences(content))?)
        };
        match attempt.await {
            Ok(mut v) => {
                // keep only quotes that really appear in the input
                v.red_flags
                    .retain(|f| !f.quote.is_empty() && input.contains(f.quote.as_str()));
                v.confidence = v.confidence.min(100);
                return Ok(v);
            }
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

fn strip_fences(s: &str) -> &str {
    let s = s.trim();
    s.strip_prefix("```json")
        .or_else(|| s.strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .map_or(s, str::trim)
}
