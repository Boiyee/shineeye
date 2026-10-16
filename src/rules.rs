use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

use crate::Lang;

#[derive(Debug, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub scam_type: String,
    pub pattern: String,
    pub weight: u8,
    #[serde(default)]
    pub hard: bool,
    pub why_en: String,
    pub why_pcm: String,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub skip_if_negated: bool,
}

pub struct Rule {
    pub pattern: Pattern,
    re: Regex,
}

/// "Never share your OTP" is a bank warning, not a request.
static NEGATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:never|not|don'?t|no|nobody|won'?t|no one)\b[^.\n]*$").unwrap()
});

impl Rule {
    pub fn find<'t>(&self, text: &'t str) -> Option<&'t str> {
        self.re
            .find_iter(text)
            .find(|m| {
                !self.pattern.skip_if_negated || {
                    let from = text[..m.start()]
                        .char_indices()
                        .rev()
                        .nth(23)
                        .map_or(0, |(i, _)| i);
                    !NEGATION.is_match(&text[from..m.start()])
                }
            })
            .map(|m| m.as_str())
    }
}

pub struct Rules(Vec<Rule>);

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub id: String,
    pub scam_type: String,
    pub quote: String,
    pub why: String,
    pub weight: u8,
    pub hard: bool,
}

impl Rules {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        Self::parse(&std::fs::read_to_string(path)?)
    }

    pub fn parse(json: &str) -> anyhow::Result<Self> {
        let patterns: Vec<Pattern> = serde_json::from_str(json)?;
        patterns
            .into_iter()
            .map(|p| {
                let re = RegexBuilder::new(&p.pattern)
                    .case_insensitive(true)
                    .build()
                    .map_err(|e| anyhow::anyhow!("pattern {}: {e}", p.id))?;
                Ok(Rule { pattern: p, re })
            })
            .collect::<anyhow::Result<_>>()
            .map(Self)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Rule> {
        self.0.iter()
    }

    pub fn scan(&self, text: &str, lang: Lang) -> Vec<Finding> {
        self.0
            .iter()
            .filter_map(|r| {
                let p = &r.pattern;
                r.find(text).map(|quote| Finding {
                    id: p.id.clone(),
                    scam_type: p.scam_type.clone(),
                    quote: quote.trim().to_owned(),
                    why: lang.pick(&p.why_en, &p.why_pcm).to_owned(),
                    weight: p.weight,
                    hard: p.hard,
                })
            })
            .collect()
    }
}
