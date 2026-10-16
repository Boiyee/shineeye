# Contributing

The quickest way to help is to add a scam pattern. It takes about five minutes and touches one file.

## Add a scam pattern

1. Open `patterns.json` and add an entry:

   ```json
   {
     "id": "fake_pos_reversal",
     "scam_type": "fake_alert",
     "pattern": "\\bpos\\b[^.\\n]{0,30}\\breversal\\b",
     "weight": 20,
     "hard": false,
     "why_en": "Fake POS reversal messages trick people into sending money back",
     "why_pcm": "Fake POS reversal message dey trick people make dem send money back",
     "examples": ["Your POS reversal of N12,000 is pending, send N2,000 to complete"]
   }
   ```

   - `pattern` is a case-insensitive Rust regex. Remember to escape backslashes in JSON (`\\b`).
   - `weight` is 10 (weak hint) to 45 (strong sign).
   - Set `hard: true` only if the signal alone means the message is at least suspicious, like asking for an OTP.
   - Add `"skip_if_negated": true` for request patterns, so bank warnings such as "never share your OTP" don't trigger them.
   - Every pattern needs at least one example. Redact personal details in examples.

2. Run `cargo test`. The test suite checks that every pattern matches its own examples and that a genuine bank alert does not trip any hard rule.

3. Open a pull request.

## Other good first issues

- Output in Yoruba, Hausa or Igbo
- EVM wallet checks (Ethereum, BSC, Base)
- More look-alike bank domains in `src/link.rs`

## Ground rules

- Never commit real phone numbers, account numbers or names.
- Run `cargo fmt` and `cargo clippy --all-targets -- -D warnings` before pushing.
