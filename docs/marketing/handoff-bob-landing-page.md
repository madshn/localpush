# Handoff: LocalPush Landing Page Build

**From:** Leah (Growth Operator)
**To:** Bob (Factory Manager)
**Date:** 2026-02-11
**Priority:** High — Mira has domain + infra nearly ready

---

## What's Ready

| Artifact | Location | Status |
|----------|----------|--------|
| **Experience brief** | `docs/marketing/landing-page-brief.md` | Complete — source of truth for all copy and page structure |
| **Stitch prototype: Landing page** | `stitch-prototypes/web/localpush_landing_stitch/` | Visual direction approved — screen.png + code.html |
| **Stitch prototype: Signup flow** | `stitch-prototypes/web/localpush_signup/` | Visual direction approved — screen.png + code.html |

---

## Build Instructions

### Use Stitch for visual direction, brief for content

The Stitch outputs nail the mood, layout, color system, and typography. But Stitch hallucinated some copy and missed some sections. **The brief is the content source of truth** — use Stitch for how things look, brief for what they say.

### Page structure (from brief)

```
1. Hero (full viewport)
2. Trust strip
3. Problem → Solution (two columns)
4. How It Works (four columns)
5. Use Cases — "What will you unlock?" (3+2 card grid)
6. "Did you know?" (4 cards + punchline)
7. Trust & Proof (badges + "Works With" logos)
8. Early Access CTA — "Join the beta." (button → modal)
9. Blog preview (3 card grid)
10. Footer
```

Sections 1–4 are in the landing Stitch export.
Sections 5–6 + the modal are in the signup Stitch export.
Sections 7–9 were not rendered by Stitch — build from brief descriptions.

### Content corrections (Stitch → brief)

These are places where the Stitch output drifted. Use the brief copy instead:

| Section | Stitch says | Brief says |
|---------|-------------|------------|
| **Hero subheadline** | Generic "unlock and automate local data from apps, logs, and databases" | "Your Mac stores incredible data — Claude Code usage, Apple Podcasts, Notes, Photos — locked away from your tools. LocalPush watches it and pushes changes to n8n, Make, Zapier, or a Google Sheet. Guaranteed delivery." |
| **Step 1 Watch** | "Monitor local directories, log files, or specific SQLite databases" | "Sources: Claude Code Stats, Apple Podcasts, Apple Notes, Apple Photos — and growing." |
| **Landing CTA heading** | "Ready to bridge your local context?" | "Join the beta." |
| **Landing CTA subtext** | "No credit card required · MIT Licensed Core" | "Prefer to wait for the open source launch? Follow on GitHub →" |
| **Footer year** | © 2024 | © 2026 |

### Signup modal flow

The signup Stitch export has this correct. Four-step flow:

1. **CTA click** → opens modal
2. **Intent capture** — "What are you most excited to try?" — 6 radio options (all correct in Stitch)
3. **Social auth** — GitHub / Discord / Google buttons (correct in Stitch)
4. **Post-auth install page** — "You're in." + brew command + DMG download + "Star on GitHub" / "Join Discord" links

The post-auth page is in the Stitch HTML as a hidden div (line 178). Toggle visibility on auth completion.

Brew command (correct in Stitch): `brew tap madshn/localpush && brew install --cask localpush`

### What to store in Supabase per signup

| Field | Source |
|-------|--------|
| Provider | Auth step (GitHub / Discord / Google) |
| Profile | Auth step (username, avatar, email) |
| Intent | Pre-auth form selection |
| Signup date | Automatic |

Rex reads this table for lead qualification. Leah reads intent data for channel strategy feedback.

---

## Design System

### From Stitch (approved)

| Token | Value |
|-------|-------|
| Background | `#0a0a0a` |
| Card surface | `#1a1a1a` |
| Primary accent | `#4f9eff` |
| Success/trust | `#4ade80` |
| Problem/error | `#f87171` |
| Text primary | `#ffffff` |
| Text muted | `#a0a0a0` |
| Border | `#222222` |
| Font display | Inter |
| Font mono | JetBrains Mono |

### Component mapping (Bob's library)

| Section | Component |
|---------|-----------|
| Hero | Launch UI hero section |
| Problem/Solution | Launch UI feature comparison or custom two-column |
| How It Works | Launch UI features section (4-column) |
| Use Cases | shadcn cards in 3+2 grid |
| "Did you know?" | shadcn cards with colored left borders |
| Trust | Launch UI social proof / badges |
| CTA | Launch UI CTA section |
| Modal | shadcn dialog + radio group + auth buttons |
| Blog | shadcn cards grid |
| Footer | Launch UI footer |
| Animations | react-bits (viewport-triggered) or Magic UI |

---

## Infrastructure (Mira is handling)

| Requirement | Solution | Status |
|-------------|----------|--------|
| Domain | localpush.dev | In progress |
| Frontend | Render static site | Ready when Bob ships |
| Backend | Supabase (social auth + site-user table) | Pending setup |
| Auth providers | GitHub, Discord, Google | Supabase OOTB |
| Blog | Static pages at /blog | Part of same deploy |
| SSL | Automatic (Render) | Automatic |

---

## What Bob Doesn't Need to Worry About

- **Blog content** — Leah produces separately, Bob just needs the card grid layout and /blog route
- **TOFU content** — demo videos, Reddit/X posts come from Leah after site is live
- **Lead qualification** — Rex handles, just needs the Supabase table to exist
- **Analytics** — PostHog wiring comes later
- **SEO meta tags** — Leah will provide after build, easy to add

---

## Definition of Done

- [ ] All 10 page sections render correctly
- [ ] Social auth works for GitHub, Discord, Google
- [ ] Intent capture stores selection in Supabase alongside user record
- [ ] Post-auth page shows correct brew command (copy-to-clipboard) + DMG link
- [ ] "Star on GitHub" and "Join Discord" links on post-auth page
- [ ] Blog route exists at /blog (even if empty initially)
- [ ] Mobile responsive
- [ ] Page loads < 2 seconds
- [ ] Footer: "Built by Right Aim" links to rightaim.ai
