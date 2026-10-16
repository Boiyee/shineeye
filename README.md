# ShineEye

**Shine your eye before you send money.**

Paste a suspicious SMS, WhatsApp message, link or crypto wallet. ShineEye tells you, in English or Pidgin, whether it is a scam, quotes the exact words that give it away, and tells you what to do next.

Built for Nigerians, open source, and powered by open-weight models.

## How it works

Every check runs two kinds of analysis and combines them:

1. **Deterministic checks.** A rule engine (`patterns.json`) catches OTP, PIN and BVN requests, fake alerts, Ponzi promises, upfront fees and more. A link inspector flags look-alike bank domains, shorteners and cheap domains. A wallet check looks up Solana addresses on-chain.
2. **An open-weight model** reads the message together with those findings and returns a structured verdict.

The final score is the higher of the two, and any hard signal (an OTP request or a fake bank domain) keeps the verdict at "suspicious" or worse. If the model is down, the rules still return a verdict.

## Run it locally

You need Rust (via [rustup](https://rustup.rs)) and an API key from any OpenAI-compatible provider. [Groq](https://console.groq.com) has a free tier.

```bash
git clone https://github.com/Boiyee && cd shineeye
cp .env.example .env    # add LLM_API_KEY and check LLM_MODEL
cargo run --release
```

Open http://localhost:3000.

### Fully local with Ollama (no API key)

```bash
ollama pull qwen2.5:7b
```

In `.env`, set `LLM_BASE_URL=http://localhost:11434/v1` and `LLM_MODEL=qwen2.5:7b`, then leave `LLM_API_KEY` empty.

## API

```http
POST /api/check
Content-Type: application/json

{ "input": "Your account will be blocked. Send the OTP we sent you", "lang": "pcm" }
```

`lang` is `en` (default) or `pcm` (Nigerian Pidgin). The response includes `verdict` (`scam`, `suspicious`, `safe`), a 0–100 `score`, `red_flags` with exact quotes, `advice`, and the raw `checks`.

## Deploy

The repo includes a `Dockerfile`. On DigitalOcean App Platform, Render or Fly.io, point the service at this repo and set `LLM_API_KEY` and `LLM_MODEL` as environment variables.

## Privacy

ShineEye does not store or log what you paste. Messages are sent to the configured model provider only to produce the verdict. Run it with Ollama to keep everything on your machine.

## Contributing

New scams appear every month. Adding one is a single-file pull request: see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT
