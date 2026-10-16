pub mod check;
pub mod link;
pub mod llm;
pub mod rules;
pub mod wallet;

use serde::Deserialize;

/// Output language for user-facing text.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    En,
    Pcm,
}

impl Lang {
    pub fn pick<'a>(self, en: &'a str, pcm: &'a str) -> &'a str {
        match self {
            Lang::En => en,
            Lang::Pcm => pcm,
        }
    }
}

pub struct Config {
    pub llm_base_url: String,
    pub llm_api_key: Option<String>,
    pub llm_model: String,
    pub solana_rpc_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        Self {
            llm_base_url: var("LLM_BASE_URL")
                .unwrap_or_else(|| "https://api.groq.com/openai/v1".into()),
            llm_api_key: var("LLM_API_KEY"),
            llm_model: var("LLM_MODEL").unwrap_or_else(|| "llama-3.3-70b-versatile".into()),
            solana_rpc_url: var("SOLANA_RPC_URL")
                .unwrap_or_else(|| "https://api.mainnet-beta.solana.com".into()),
        }
    }
}

pub struct AppState {
    pub cfg: Config,
    pub rules: rules::Rules,
    pub http: reqwest::Client,
}
