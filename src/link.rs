use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use url::Url;

use crate::Lang;

/// Brand token -> official domains. A host that mentions the brand but is not
/// on (or under) an official domain is treated as a look-alike.
const BRANDS: &[(&str, &[&str])] = &[
    ("gtbank", &["gtbank.com", "gtco.com"]),
    ("gtb", &["gtbank.com", "gtco.com"]),
    ("gtco", &["gtco.com", "gtbank.com"]),
    ("zenith", &["zenithbank.com"]),
    ("firstbank", &["firstbanknigeria.com", "firstbankgroup.com"]),
    ("accessbank", &["accessbankplc.com"]),
    ("uba", &["ubagroup.com"]),
    ("fidelity", &["fidelitybank.ng"]),
    ("wema", &["wemabank.com", "alat.ng"]),
    ("ecobank", &["ecobank.com"]),
    ("sterling", &["sterling.ng"]),
    ("stanbic", &["stanbicibtcbank.com"]),
    ("polaris", &["polarisbanklimited.com"]),
    ("fcmb", &["fcmb.com"]),
    ("opay", &["opayweb.com", "opay.ng"]),
    ("palmpay", &["palmpay.com"]),
    ("kuda", &["kuda.com"]),
    ("moniepoint", &["moniepoint.com"]),
    ("cbn", &["cbn.gov.ng"]),
    ("nibss", &["nibss-plc.com.ng"]),
    ("efcc", &["efcc.gov.ng"]),
    ("bvn", &[]),
    ("nin", &[]),
];

const SHORTENERS: &[&str] = &[
    "bit.ly",
    "tinyurl.com",
    "t.co",
    "cutt.ly",
    "rb.gy",
    "is.gd",
    "shorturl.at",
    "tiny.cc",
    "ow.ly",
    "s.id",
    "t.ly",
    "rebrand.ly",
    "goo.su",
    "shorturl.asia",
];

const RISKY_TLDS: &[&str] = &[
    "xyz", "top", "icu", "buzz", "tk", "ml", "ga", "cf", "gq", "click", "online", "site", "live",
    "vip", "shop", "rest", "cfd", "sbs", "cyou", "monster", "lol",
];

static URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    let tlds =
        "com|ng|net|org|io|co|me|app|ly|cc|info|biz|link|".to_owned() + &RISKY_TLDS.join("|");
    Regex::new(&format!(
        r"(?i)\b(?:https?://|www\.)[^\s<>()]+|\b(?:[a-z0-9-]+\.)+(?:{tlds})(?:/[^\s<>()]*)?\b"
    ))
    .unwrap()
});

#[derive(Debug, Clone, Serialize)]
pub struct LinkFinding {
    pub url: String,
    pub host: String,
    pub flags: Vec<String>,
    pub weight: u8,
    pub hard: bool,
}

pub fn extract(text: &str) -> Vec<&str> {
    URL_RE
        .find_iter(text)
        .map(|m| {
            m.as_str()
                .trim_end_matches(['.', ',', '!', '?', ';', ':', '"', '\''])
        })
        .collect()
}

pub fn inspect(text: &str, lang: Lang) -> Vec<LinkFinding> {
    extract(text)
        .into_iter()
        .filter_map(|raw| inspect_one(raw, lang))
        .collect()
}

fn inspect_one(raw: &str, lang: Lang) -> Option<LinkFinding> {
    let parsed = if raw.contains("://") {
        Url::parse(raw)
    } else {
        Url::parse(&format!("https://{raw}"))
    }
    .ok()?;
    let host = parsed.host_str()?.trim_start_matches("www.").to_lowercase();

    let mut flags = Vec::new();
    let (mut weight, mut hard) = (0u8, false);
    let mut flag = |en: &str, pcm: &str, w: u8, h: bool| {
        flags.push(lang.pick(en, pcm).to_owned());
        weight = weight.saturating_add(w);
        hard |= h;
    };

    if host.parse::<std::net::IpAddr>().is_ok() {
        flag(
            "Link goes to a raw IP address, not a real website name",
            "Dis link na IP address, no be correct website name",
            30,
            true,
        );
    }
    if host.split('.').any(|l| l.starts_with("xn--")) {
        flag(
            "Link uses look-alike characters to copy a real name",
            "Dis link dey use fake letters wey resemble real name",
            30,
            true,
        );
    }
    if SHORTENERS.contains(&host.as_str()) {
        flag(
            "Shortened link hides where it really goes",
            "Dem shorten dis link make you no see where e dey carry you",
            15,
            false,
        );
    }
    if let Some(tld) = host.rsplit('.').next().filter(|t| RISKY_TLDS.contains(t)) {
        flag(
            &format!("Cheap .{tld} domain, common in scam sites"),
            &format!(".{tld} domain cheap well well, scammers like am"),
            15,
            false,
        );
    }
    let tokens: Vec<&str> = host.split(['.', '-']).collect();
    for (brand, official) in BRANDS {
        let mentions = tokens.contains(brand) || (brand.len() >= 5 && host.contains(brand));
        let is_official = official
            .iter()
            .any(|d| host == *d || host.ends_with(&format!(".{d}")));
        if mentions && !is_official {
            flag(
                &format!("Uses \"{brand}\" in the address but is not the official site"),
                &format!("E carry \"{brand}\" name but e no be the real website"),
                40,
                true,
            );
            break;
        }
    }

    Some(LinkFinding {
        url: raw.to_owned(),
        host,
        flags,
        weight,
        hard,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags_for(text: &str) -> LinkFinding {
        inspect(text, Lang::En).remove(0)
    }

    #[test]
    fn lookalike_bank_domain_is_hard() {
        let f = flags_for("Update your BVN now: gtb-verify.xyz/login");
        assert!(f.hard);
        assert!(f.weight >= 55);
    }

    #[test]
    fn official_domain_is_clean() {
        let f = flags_for("Visit https://www.gtbank.com/personal");
        assert!(f.flags.is_empty(), "{:?}", f.flags);
    }

    #[test]
    fn shortener_flagged() {
        assert_eq!(flags_for("click bit.ly/3xYz").weight, 15);
    }

    #[test]
    fn ignores_plain_numbers() {
        assert!(extract("Acct: 01**89 CR: NGN150,000.00 Bal: 2.5").is_empty());
    }
}
