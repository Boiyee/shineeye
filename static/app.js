const GITHUB = "https://github.com/Boiyee"; // set after creating the repo

const SAMPLES = {
  alert: "Acct: 01**89 CR: NGN150,000.00 Bal: NGN151,200.00. Pls I mistakenly sent 150k to your account, kindly refund the balance to 0803*****12 urgently.",
  otp: "GTBank: Your account will be blocked today due to BVN mismatch. Click the link below to verify: gtb-verify.xyz or send the OTP we sent to you.",
  crypto: "Invest 50k, get 150k in 7 days! Guaranteed returns. Send USDT to MASi45ub7Qe4ZE36UT5G6cU4ud8Fhhe4deS4F3cw9KTA now now.",
};

const T = {
  en: {
    lede: "Paste any message, link or wallet. Know if it's a scam before you send money.",
    check: "Check it", checking: "Checking", flags: "Red flags", todo: "What to do", raw: "Show raw checks",
    scam: "Scam", suspicious: "Suspicious", safe: "Looks safe", none: "No red flags found.",
    model: "Rules + open model", rules: "Rules only", err: "Couldn't check right now. Try again.",
  },
  pcm: {
    lede: "Paste any message, link or wallet. Know if na scam before you send money.",
    check: "Check am", checking: "We dey check", flags: "Red flags", todo: "Wetin to do", raw: "Show raw checks",
    scam: "Na scam", suspicious: "E no clear", safe: "E be like say e safe", none: "We no see red flag.",
    model: "Rules + open model", rules: "Rules only", err: "We no fit check am now. Try again.",
  },
};

const $ = (s) => document.querySelector(s);
const form = $("#form"), input = $("#input"), go = $("#go"), verdict = $("#verdict"), err = $("#err");
let lang = "en";
let lastInput = "";

for (const id of ["#gh-nav", "#gh-cta", "#gh-foot"]) $(id).href = GITHUB;
$("#gh-contrib").href = `${GITHUB}/blob/main/CONTRIBUTING.md`;

function applyLang() {
  document.documentElement.lang = lang === "pcm" ? "pcm" : "en";
  document.querySelectorAll("[data-i18n]").forEach((el) => (el.textContent = T[lang][el.dataset.i18n]));
  document.querySelectorAll(".seg button").forEach((b) => b.setAttribute("aria-pressed", b.dataset.lang === lang));
}

document.querySelectorAll(".seg button").forEach((b) =>
  b.addEventListener("click", () => {
    if (b.dataset.lang === lang) return;
    lang = b.dataset.lang;
    applyLang();
    if (lastInput && !verdict.hidden) check(lastInput);
  })
);

document.querySelectorAll(".chip").forEach((c) =>
  c.addEventListener("click", () => {
    input.value = SAMPLES[c.dataset.sample];
    check(input.value);
  })
);

form.addEventListener("submit", (e) => {
  e.preventDefault();
  if (input.value.trim()) check(input.value);
});

input.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) form.requestSubmit();
});

async function check(text) {
  lastInput = text;
  err.hidden = true;
  form.classList.add("busy");
  go.disabled = true;
  go.firstElementChild.textContent = T[lang].checking;
  try {
    const r = await fetch("/api/check", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ input: text, lang }),
    });
    const d = await r.json();
    if (!r.ok) throw new Error(d.error || r.status);
    render(d);
  } catch (e) {
    err.textContent = T[lang].err;
    err.hidden = false;
  } finally {
    form.classList.remove("busy");
    go.disabled = false;
    go.firstElementChild.textContent = T[lang].check;
  }
}

function el(tag, text, cls) {
  const n = document.createElement(tag);
  if (text) n.textContent = text;
  if (cls) n.className = cls;
  return n;
}

function render(d) {
  verdict.dataset.v = d.verdict;
  $("#v-label").textContent = T[lang][d.verdict];
  $("#v-sum").textContent = d.summary;
  $("#v-meta").textContent = `${d.model_used ? T[lang].model : T[lang].rules} · ${d.latency_ms} ms`;

  const flags = $("#flags");
  flags.replaceChildren(
    ...(d.red_flags.length
      ? d.red_flags.map((f, i) => {
          const li = el("li");
          li.style.animationDelay = `${120 + i * 90}ms`;
          li.append(el("q", f.quote), el("span", f.why));
          return li;
        })
      : [el("li", T[lang].none, "none")])
  );
  $("#advice").replaceChildren(...d.advice.map((a) => el("li", a)));
  $("#raw").textContent = JSON.stringify(d.checks, null, 2);

  const wasHidden = verdict.hidden;
  verdict.hidden = false;
  // restart the entrance animation on every result
  verdict.style.animation = "none";
  void verdict.offsetWidth;
  verdict.style.animation = "";
  countUp($("#score"), d.score);
  if (wasHidden) verdict.scrollIntoView({ behavior: "smooth", block: "nearest" });
}

function countUp(node, to) {
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) return (node.textContent = to);
  const start = performance.now(), dur = 700;
  const tick = (t) => {
    const p = Math.min(1, (t - start) / dur);
    node.textContent = Math.round(to * (1 - Math.pow(1 - p, 3)));
    if (p < 1) requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

// Android share target: /?text=...&url=...
const shared = new URLSearchParams(location.search);
const sharedText = [shared.get("title"), shared.get("text"), shared.get("url")].filter(Boolean).join(" ");
if (sharedText) {
  input.value = sharedText;
  history.replaceState(null, "", "/");
  check(sharedText);
}

applyLang();
if ("serviceWorker" in navigator) navigator.serviceWorker.register("/sw.js").catch(() => {});
