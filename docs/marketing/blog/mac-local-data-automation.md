# Your Mac Stores Incredible Data About Your Life. Your Automation Stack Can't Touch It.

**Target keyword:** mac data automation
**Secondary keywords:** local data to webhook, mac local data sync, apple data automation
**Audience:** Segments 1 + 3 — automation power users + privacy-conscious prosumers
**Hook:** The Data Liberation Angle
**Status:** Draft — Wk 12 2026

---

**Meta description:** Your Mac has years of data: notes, photos, podcasts, app usage, productivity stats. Most of it is completely inaccessible to your automation stack. LocalPush bridges the gap.

---

Think about what your Mac knows about you.

It knows every note you've taken in Apple Notes. Every podcast you've listened to and how far you got. Every photo you've added to your library. Every session you've spent in your coding tools, with token-level detail. App usage patterns, file creation timestamps, local databases maintained by apps you've been using for years.

This data sits on your machine, updated continuously, and is essentially invisible to every automation tool in your stack. Your n8n instance can't see it. Make can't reach it. Zapier doesn't know it exists.

The gap isn't a design flaw — it's a reasonable boundary. Automation platforms live in the cloud. Your local data lives on your Mac. They don't naturally talk to each other.

LocalPush is a bridge.

## What the bridge looks like in practice

LocalPush is a macOS menu bar utility (free, open source, MIT licensed) that watches local data sources and delivers events to any webhook endpoint. You configure a source, configure a target, and LocalPush handles the rest.

Practically, this means:

**Apple Notes → Notion database**
Add the Apple Notes source in LocalPush. Configure your Notion integration webhook (via n8n or Make) as the target. Every new note or significant edit fires a payload to your workflow — which can write the note to a Notion database, tag it, run it through an LLM for summarization, or trigger any downstream action.

**Apple Podcasts → listening tracker**
Configure the Podcasts source. Every completed episode fires a payload with episode metadata — title, podcast name, duration, timestamp. Route it through n8n to an Airtable or a personal Obsidian file. Now you have a permanent record of everything you've listened to, in a format your other tools can query.

**Claude Code Stats → cost dashboard**
For developers: LocalPush reads your local Claude Code usage stats and pushes per-session data to any webhook. Token counts, model usage, session duration, estimated cost. Route to a Google Sheet or a Postgres database. Now you have a spending dashboard with session-level granularity.

**Any watched file → any workflow**
Beyond built-in sources, LocalPush can watch any file on disk. If an app you use exports to a local file, LocalPush can watch it and fire immediately on changes.

## The transparency guarantee

Before LocalPush sends anything, it shows you exactly what gets sent.

Not a schema description. Not a sample dataset. Your actual current data — the real JSON payload that will be delivered to your webhook. You can inspect it, decide if you're comfortable with it, and only then enable delivery.

This isn't just a privacy checkbox. It makes integration faster: you're building your n8n workflow against real data, not guessing at field names.

## Guaranteed delivery, not fire-and-forget

A naive local webhook sender has an obvious failure mode: if the target is temporarily unreachable, the event is lost. If your Mac is asleep when a change happens, the event is missed.

LocalPush uses a write-ahead log (WAL) pattern borrowed from database design. Every event is written to a local journal before delivery is attempted. The journal survives crashes, sleep cycles, and restarts. When the target is reachable, delivery completes — no gaps, no silent failures.

This matters for data you care about. A podcast listening log with random gaps is less useful. A cost tracker that misses sessions is misleading. Guaranteed delivery makes the data reliable enough to act on.

## Who this is for

**Automation builders** who've wanted to trigger n8n or Make from local events but couldn't bridge the gap. LocalPush makes local data a first-class trigger source.

**Quantified-self practitioners** who track personal metrics but have hit walls trying to get Apple ecosystem data into their tools. LocalPush is the extraction layer.

**Privacy-conscious power users** who want automation without sending raw data to cloud services. LocalPush is local-first: events are processed on your Mac, delivered over a connection you control, to an endpoint you choose. Open source so you can verify.

**Developers** who want to instrument their own workflow — tracking Claude Code costs, monitoring build outputs, watching project directories for changes.

## Getting started

```bash
brew tap madshn/localpush && brew install --cask localpush
```

Open the menu bar app. Add a source — you'll see your real data immediately in the preview pane. Configure a webhook target. Enable delivery.

The GitHub repo is at [github.com/madshn/localpush](https://github.com/madshn/localpush). The README documents all current sources and the payload format for each.

---

*LocalPush is a free, open source macOS utility. MIT licensed. No cloud account required.*
