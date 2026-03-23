# Track Your Claude Code Token Spend Automatically with LocalPush + n8n

**Target keyword:** claude code token tracking
**Secondary keywords:** claude usage dashboard, claude code stats, track claude api costs
**Audience:** Segment 2 — Developers tracking metrics
**Hook:** The Claude Stats Hook
**Status:** Draft — Wk 12 2026

---

**Meta description:** Claude Code buries your token usage in local files. LocalPush reads those stats and pushes them to n8n automatically — build a real usage dashboard in under 10 minutes.

---

If you use Claude Code heavily, you probably have a vague sense that you're spending a lot on tokens. Maybe you've poked around in `~/.claude/` and found the stats files. Maybe you've run `claude --usage` a few times. But you almost certainly don't have a live dashboard tracking your spend session by session.

That's because Claude Code stores your usage data locally — which is great for privacy, but means your automation stack can't see it unless something bridges the gap.

LocalPush is that bridge.

## Where Claude Code stores your stats

Claude Code writes usage data to a local stats file under your home directory. The file tracks token counts, session durations, model usage, and cost estimates — updated continuously as you work.

The data is there. It's just sitting on disk where your n8n instance can't reach it.

## The setup: LocalPush → n8n → dashboard

The pipeline has three parts:

**1. LocalPush watches the Claude Code stats file.**

In LocalPush, add the Claude Code Stats source. It reads your local usage file and emits a structured payload every time the stats update — typically at the end of each session or when you switch contexts.

Before enabling delivery, LocalPush shows you exactly what gets sent: the raw JSON with your token counts, model, duration, and cost. Nothing leaves your machine without you seeing it first.

**2. LocalPush pushes to your n8n webhook.**

Configure a webhook node in n8n, paste the URL into LocalPush, and you're connected. LocalPush will push new stat events in real time, with guaranteed delivery — if your n8n instance is temporarily down, events are queued locally and delivered when it comes back.

**3. n8n writes to your dashboard of choice.**

From n8n, you can route the data anywhere: a Notion database, an Airtable, a Google Sheet, a Postgres table, a Grafana dashboard. The payload is clean JSON, so any downstream node can consume it.

A simple n8n workflow:

```
Webhook (LocalPush trigger)
  → Set node (extract: model, input_tokens, output_tokens, cost_usd, session_id, timestamp)
  → Notion / Airtable / Postgres (append row)
```

That's it. One webhook, one transform, one destination.

## What you can track once it's running

Once the pipeline is live, you have per-session data to work with:

- **Daily spend** — how much you're burning per day, broken down by model
- **Session efficiency** — which sessions are token-heavy vs. lightweight
- **Model mix** — how often you're hitting Sonnet vs. Opus vs. Haiku
- **Week-over-week trends** — are you speeding up or getting more efficient?
- **Project attribution** — if you tag sessions, you can break spend down by project

For teams or solo operators trying to make sense of AI tooling costs, this is the feedback loop that's been missing.

## Why not just use the Claude API dashboard?

Anthropic's usage dashboard shows aggregate spend across API keys, but Claude Code's local stats have session-level granularity that the API dashboard doesn't expose. If you want to understand *which work* is costing what — not just that you spent $X this month — local stats are richer.

There's also the latency question. The API dashboard updates with some delay. LocalPush fires within seconds of your session completing.

## Installation

LocalPush is free, open source (MIT), and installs via Homebrew:

```bash
brew tap madshn/localpush && brew install --cask localpush
```

After installing, open the menu bar app, go to Sources, and add Claude Code Stats. The UI shows you a preview of your actual data before you configure any delivery. Connect your n8n webhook URL, enable the source, and events start flowing.

Source code and documentation at [github.com/madshn/localpush](https://github.com/madshn/localpush).

---

*LocalPush is a free, open source macOS utility. MIT licensed.*
