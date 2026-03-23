# n8n Local File Trigger: Watch Files on Your Mac and Fire Webhooks Instantly

**Target keyword:** n8n local trigger
**Secondary keywords:** n8n file watcher, n8n local file, n8n webhook local data
**Audience:** Segment 1 — n8n power users
**Hook:** The Cron Job Killer / Data Liberation combo
**Status:** Draft — Wk 12 2026

---

**Meta description:** n8n can't see files on your local machine. LocalPush solves this — it watches files and local data sources on macOS and fires to any n8n webhook trigger instantly, with guaranteed delivery.

---

n8n is excellent at connecting services. But it has a blind spot: your local machine.

If you want to trigger an n8n workflow when a file changes on your Mac, or when a local app updates its database, you're stuck. n8n cloud can't reach into your filesystem. Even self-hosted n8n can only reach files on the server it runs on — not your laptop.

LocalPush fills that gap. It's a macOS menu bar app that watches local files and data sources and delivers events to any webhook, including n8n's webhook trigger node.

## The architecture

The setup is straightforward:

```
Local Mac
  └── LocalPush (watches files)
        └── HTTP POST → n8n Webhook Trigger Node
              └── your workflow continues...
```

LocalPush runs as a background menu bar process on your Mac. You configure it with:
1. A **source** — what to watch (a file, a CSV, a SQLite database, Claude Code stats, Apple Notes, etc.)
2. A **target** — where to deliver (your n8n webhook URL)

When the source changes, LocalPush fires immediately using native macOS FSEvents — no polling interval, no lag. The event arrives at your n8n webhook within seconds of the change on disk.

## Setting up the n8n side

In n8n, add a **Webhook** node as your trigger:

1. Set the HTTP method to POST
2. Copy the webhook URL (either test URL or production URL)
3. Paste it into LocalPush as your target endpoint

That's the entire n8n configuration. The payload from LocalPush arrives as a standard JSON body, so any downstream node can parse it with `{{ $json.fieldName }}`.

Example payload structure (varies by source):

```json
{
  "source": "csv_watcher",
  "event": "row_added",
  "timestamp": "2026-03-16T09:14:22Z",
  "data": {
    "row_index": 142,
    "columns": {
      "name": "example",
      "value": "123"
    }
  }
}
```

From there, build your workflow as normal: transform the data, write to a database, send a notification, trigger another service.

## Guaranteed delivery — even when n8n is down

One of the problems with naive file watcher → webhook setups is brittleness. If your n8n instance restarts, or your network drops, or the webhook endpoint is temporarily unreachable, the event is lost.

LocalPush uses a write-ahead log (WAL) pattern. Every event is written to a local journal on your Mac before delivery is attempted. If delivery fails, LocalPush retries when the target becomes reachable again. Your Mac can restart, n8n can go down for maintenance, your wifi can drop — the event survives and delivers when everything comes back.

This matters if you're using n8n to process financial records, audit logs, or any data where "I think it probably went through" isn't good enough.

## What you can watch

Current sources in LocalPush:

- **Any file on disk** — watch a single file for changes; fires on any write
- **CSV files** — tracks new rows appended to a CSV
- **Claude Code Stats** — per-session token usage and cost data
- **Claude Sessions** — session metadata from `~/.claude/`
- **Apple Podcasts** — listen history and episode data
- **Apple Notes** — new or modified notes
- **Apple Photos** — new photos added to the library

More sources are in development. The architecture is extensible — if you want to add a source, the repository is MIT licensed.

## Radical transparency: see your data before sending

Before LocalPush sends anything, it shows you your real data. Not a schema description, not a sample. Your actual current data — the exact JSON payload that will be delivered to your n8n webhook.

This matters for two reasons. First, it's a privacy guarantee: you know exactly what leaves your machine before enabling any source. Second, it makes webhook integration faster — you can see the payload structure immediately and build your n8n workflow against real data, not guesses.

## Getting started

```bash
brew tap madshn/localpush && brew install --cask localpush
```

Open the menu bar app, add a source, paste your n8n webhook URL, and enable delivery. You should see the first event in your n8n execution log within seconds of a change on the watched source.

Full documentation and source code: [github.com/madshn/localpush](https://github.com/madshn/localpush)

---

*LocalPush is a free, open source macOS utility. MIT licensed. Works with any webhook endpoint — n8n, Make, Zapier, or your own server.*
