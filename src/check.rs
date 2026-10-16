use std::{sync::Arc, time::Instant};

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    link::{self, LinkFinding},
    llm::{self, Flag, ModelVerdict, Verdict},
    rules::Finding,
    wallet::{self, WalletFinding},
    AppState, Lang,
};

const MAX_INPUT: usize = 4000;
const HARD_FLOOR: u8 = 60;

#[derive(Deserialize)]
pub struct CheckRequest {
    pub input: String,
    #[serde(default)]
    pub lang: Lang,
}

#[derive(Serialize)]
pub struct Checks {
    pub rules: Vec<Finding>,
    pub links: Vec<LinkFinding>,
    pub wallet: Option<WalletFinding>,
}

#[derive(Serialize)]
pub struct CheckResponse {
    pub kind: &'static str,
    pub verdict: Verdict,
    pub score: u8,
    pub scam_type: String,
    pub summary: String,
    pub red_flags: Vec<Flag>,
    pub advice: Vec<String>,
    pub checks: Checks,
    pub model_used: bool,
    pub latency_ms: u128,
}

type ApiError = (StatusCode, Json<serde_json::Value>);

pub async fn handler(
    State(st): State<Arc<AppState>>,
    Json(req): Json<CheckRequest>,
) -> Result<Json<CheckResponse>, ApiError> {
    let started = Instant::now();
    let input = req.input.trim();
    if input.is_empty() || input.len() > MAX_INPUT {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("input must be 1-{MAX_INPUT} characters") })),
        ));
    }
    let lang = req.lang;

    let rules = st.rules.scan(input, lang);
    let links = link::inspect(input, lang);
    let wallet = wallet::check(&st.http, &st.cfg.solana_rpc_url, input, lang).await;
    let checks = Checks {
        rules,
        links,
        wallet,
    };

    let evidence = json!(checks);
    // a local model (e.g. Ollama) needs no key; hosted ones do
    let use_model = st.cfg.llm_api_key.is_some() || !st.cfg.llm_base_url.contains("groq.com");
    let model = if use_model {
        llm::judge(&st.http, &st.cfg, input, &evidence, lang)
            .await
            .inspect_err(|e| eprintln!("model unavailable, rules-only verdict: {e}"))
            .ok()
    } else {
        None
    };

    Ok(Json(build(input, lang, checks, model, started)))
}

fn build(
    input: &str,
    lang: Lang,
    checks: Checks,
    model: Option<ModelVerdict>,
    started: Instant,
) -> CheckResponse {
    let kind = detect_kind(input, &checks);
    let hard = checks.rules.iter().any(|f| f.hard) || checks.links.iter().any(|l| l.hard);
    let rule_sum: u32 = checks.rules.iter().map(|f| f.weight as u32).sum::<u32>()
        + checks.links.iter().map(|l| l.weight as u32).sum::<u32>()
        + checks.wallet.as_ref().map_or(0, |w| w.weight as u32);
    let rule_score = rule_sum.min(100) as u8;

    let model_score = model.as_ref().map(|m| {
        let c = m.confidence as u32;
        (match m.verdict {
            Verdict::Scam => 70 + c * 30 / 100,
            Verdict::Suspicious => 40 + c * 25 / 100,
            Verdict::Safe => (100 - c) * 35 / 100,
        }) as u8
    });

    let mut score = model_score.unwrap_or(0).max(rule_score);
    if hard {
        score = score.max(HARD_FLOOR);
    }
    let verdict = match score {
        70.. => Verdict::Scam,
        40.. => Verdict::Suspicious,
        _ => Verdict::Safe,
    };

    // deterministic findings first (links, wallet, rules), then model flags on new text
    let flag = |quote: &str, why: String| Flag {
        quote: quote.to_owned(),
        why,
    };
    let mut red_flags: Vec<Flag> = checks
        .links
        .iter()
        .filter(|l| !l.flags.is_empty())
        .map(|l| flag(&l.url, l.flags.join(". ")))
        .chain(
            checks
                .wallet
                .iter()
                .map(|w| flag(&w.address, w.flags.join(". "))),
        )
        .chain(checks.rules.iter().map(|f| flag(&f.quote, f.why.clone())))
        .collect();
    let model_used = model.is_some();
    let (scam_type, summary, advice) = match model {
        Some(m) => {
            for f in m.red_flags {
                if !red_flags.iter().any(|r| overlaps(input, &r.quote, &f.quote)) {
                    red_flags.push(f);
                }
            }
            (m.scam_type, m.summary, m.advice)
        }
        None => {
            let st = checks
                .rules
                .iter()
                .max_by_key(|f| f.weight)
                .map_or("none", |f| f.scam_type.as_str())
                .to_owned();
            (
                st,
                fallback_summary(&verdict, lang).into(),
                fallback_advice(&verdict, lang),
            )
        }
    };
    let advice = if advice.is_empty() {
        fallback_advice(&verdict, lang)
    } else {
        advice
    };
    red_flags.truncate(5);

    CheckResponse {
        kind,
        verdict,
        score,
        scam_type,
        summary,
        red_flags,
        advice,
        checks,
        model_used,
        latency_ms: started.elapsed().as_millis(),
    }
}

/// Two quotes overlap if their first occurrences in the input share any bytes.
fn overlaps(input: &str, a: &str, b: &str) -> bool {
    let span = |q: &str| input.find(q).map(|s| s..s + q.len());
    match (span(a), span(b)) {
        (Some(x), Some(y)) => x.start < y.end && y.start < x.end,
        _ => a.eq_ignore_ascii_case(b),
    }
}

fn detect_kind(input: &str, checks: &Checks) -> &'static str {
    let words = input.split_whitespace().count();
    match () {
        _ if checks.wallet.is_some() && words <= 2 => "wallet",
        _ if !checks.links.is_empty() && words <= 2 => "link",
        _ if words <= 3 && input.chars().filter(char::is_ascii_digit).count() == 10 => "account",
        _ => "message",
    }
}

fn fallback_summary(v: &Verdict, lang: Lang) -> &'static str {
    match v {
        Verdict::Scam => lang.pick(
            "This has strong signs of a scam.",
            "Dis one get strong sign say na scam.",
        ),
        Verdict::Suspicious => lang.pick(
            "Some signs here are worrying. Be careful.",
            "Some things for here no clear. Shine your eye.",
        ),
        Verdict::Safe => lang.pick(
            "No common scam signs found.",
            "We no see the normal scam signs.",
        ),
    }
}

fn fallback_advice(v: &Verdict, lang: Lang) -> Vec<String> {
    let items: &[(&str, &str)] = match v {
        Verdict::Safe => &[(
            "Still never share your OTP, PIN or BVN with anyone",
            "Even so, no ever give anybody your OTP, PIN or BVN",
        )],
        _ => &[
            (
                "Do not reply, click links or send money",
                "No reply, no click link, no send money",
            ),
            (
                "Call your bank using the number on the back of your card",
                "Call your bank with the number wey dey back of your card",
            ),
            (
                "Report the number or account to your bank",
                "Report the number or account give your bank",
            ),
        ],
    };
    items
        .iter()
        .map(|(en, pcm)| lang.pick(en, pcm).to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;

    fn rules() -> Rules {
        Rules::load(concat!(env!("CARGO_MANIFEST_DIR"), "/patterns.json")).unwrap()
    }

    fn rules_only(text: &str) -> CheckResponse {
        let checks = Checks {
            rules: rules().scan(text, Lang::En),
            links: link::inspect(text, Lang::En),
            wallet: None,
        };
        build(text, Lang::En, checks, None, Instant::now())
    }

    #[test]
    fn otp_request_is_at_least_suspicious_without_model() {
        let r = rules_only(
            "Your GTB account will be blocked today. Send the OTP we sent to you to reactivate.",
        );
        assert!(r.score >= HARD_FLOOR, "score {}", r.score);
        assert!(!matches!(r.verdict, Verdict::Safe));
        assert!(!r.red_flags.is_empty());
    }

    #[test]
    fn plain_message_is_safe() {
        let r = rules_only("Hi, are we still meeting at the library by 4pm?");
        assert!(matches!(r.verdict, Verdict::Safe), "score {}", r.score);
    }

    #[test]
    fn hard_floor_beats_safe_model() {
        let text = "Kindly send your BVN and PIN to confirm";
        let checks = Checks {
            rules: rules().scan(text, Lang::En),
            links: vec![],
            wallet: None,
        };
        let model = ModelVerdict {
            verdict: Verdict::Safe,
            confidence: 90,
            scam_type: "none".into(),
            summary: String::new(),
            red_flags: vec![],
            advice: vec![],
        };
        let r = build(text, Lang::En, checks, Some(model), Instant::now());
        assert!(r.score >= HARD_FLOOR);
    }
}
