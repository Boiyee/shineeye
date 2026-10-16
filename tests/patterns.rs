use shineeye::{rules::Rules, Lang};

fn rules() -> Rules {
    Rules::load(concat!(env!("CARGO_MANIFEST_DIR"), "/patterns.json"))
        .expect("patterns.json must parse")
}

#[test]
fn every_pattern_matches_its_examples() {
    let rules = rules();
    for rule in rules.iter() {
        let p = &rule.pattern;
        assert!(!p.examples.is_empty(), "{} has no examples", p.id);
        for ex in &p.examples {
            assert!(
                rule.find(ex).is_some(),
                "{} does not match its example: {ex:?}",
                p.id
            );
        }
    }
}

#[test]
fn bank_warnings_are_not_requests() {
    let genuine = "Acct:01**89 Amt:NGN5,000.00 CR Desc:TRF FROM JOHN Bal:NGN25,300.00. \
                   Never share your OTP, PIN or BVN with anyone.";
    let hard: Vec<_> = rules()
        .scan(genuine, Lang::En)
        .into_iter()
        .filter(|f| f.hard)
        .collect();
    assert!(hard.is_empty(), "false hard hits: {hard:?}");
}

#[test]
fn pidgin_explanations_exist() {
    for rule in rules().iter() {
        assert!(
            !rule.pattern.why_pcm.trim().is_empty(),
            "{} missing why_pcm",
            rule.pattern.id
        );
    }
}
