# LocalPush Landing Page Experience Brief

**Author:** Leah (Growth Operator)
**Date:** 2026-02-11
**Status:** Approved — visual direction confirmed, handoff to Bob ready
**Handoff to:** Bob (build), Mira (deploy + domain), Rex (lead pickup)

---

## Purpose

This is the conversion surface for localpush.dev. Every TOFU channel (Reddit, HN, X, YouTube) drives here. The page must convert a curious visitor into one of three outcomes:

1. **Early tester signup** (primary) — sign up via social auth, get Homebrew/DMG install access
2. **GitHub follow** (secondary) — follow the project on GitHub ahead of OSS launch
3. **Return visitor** (tertiary) — bookmarks the page, reads a blog post, comes back later

**Beta framing:** The project is currently in testing. The repo is private. The landing page teases that LocalPush launches as open source on GitHub soon, and positions signup as "be an early tester." Upon signup, users get the Homebrew tap command or direct DMG download.

**Backend:** Supabase with social auth providers (GitHub, Discord, Google — developer-native identity). Signing in with GitHub or Discord feels like joining a community, not filling out a form. Rex reads the site-user table for lead pickup. Mira deploys on Render as static frontend.

**Why social auth over email input:** GitHub sign-in gives us their developer profile. Discord gives us community identity. Google is the universal fallback. All three feel higher-status than typing an email into a box — it's a "join" action, not a "subscribe" action.

---

## Visitor Arrival Scenarios

Every section of this page is designed for specific visitors arriving from specific content. This is not a generic product page — it's a landing strip for known TOFU flows.

### Scenario A: "I saw a demo"

**Source:** YouTube Short, X/Twitter video, GIF on Reddit
**Content example:** 30-second screen recording showing LocalPush pushing Claude Code stats to an n8n dashboard in real time.
**Visitor mindset:** "That looked cool, I want to try it."
**What they need from the page:** Install command, confirmation it works with their stack (n8n/Make/Zapier), how fast to first delivery.
**Conversion path:** Hero → Install command → Download

### Scenario B: "I read about reliable local triggers"

**Source:** Reddit r/n8n, r/selfhosted post about "guaranteed delivery file watcher"
**Content example:** Forum post explaining why cron jobs fail silently and how LocalPush uses WAL pattern for crash-safe delivery.
**Visitor mindset:** "My cron job is unreliable. Does this actually solve it?"
**What they need from the page:** Technical credibility, reliability mechanism, proof it survives crashes/reboots.
**Conversion path:** Hero → How It Works (WAL explainer) → Download

### Scenario C: "I saw the transparency angle"

**Source:** Hacker News post, privacy-focused Reddit thread
**Content example:** Post about "before LocalPush sends anything, it shows you YOUR real data"
**Visitor mindset:** "What exactly does this send? Can I verify?"
**What they need from the page:** Screenshots of the transparency preview, open source signal, data control messaging.
**Conversion path:** Hero → Transparency section → GitHub link → Download

### Scenario D: "I want to track my Claude usage"

**Source:** r/ClaudeAI, X/Twitter #buildinpublic
**Content example:** Post showing a personal Claude Code token spend dashboard powered by LocalPush + n8n.
**Visitor mindset:** "I use Claude Code daily. I want this dashboard."
**What they need from the page:** Claude Code specific use case, setup time ("5 minutes"), what they'll get.
**Conversion path:** Hero → Use Cases (Claude Stats) → Sign up as early tester

**Demographic note:** This is a real, identifiable tribe — AI power users who obsess over their own usage data. They track tokens, compare models, share dashboards. They will screenshot your product and post it. High-engagement, high-signal audience.

### Scenario E: "I'm building local AI infrastructure"

**Source:** X/Twitter threads about OpenClaw/Moltbot, r/selfhosted, HN discussions about Mac Mini AI farms, #buildinpublic
**Content example:** Post about "I built something better than OpenClaw — an entire AI agent team running on my self-hosted n8n, powered by local Mac data via LocalPush."
**Visitor mindset:** "I want my AI agents to access my local data safely. I'm already running self-hosted infra."
**What they need from the page:** How LocalPush fits into a self-hosted AI stack, security/transparency story, guaranteed delivery to their own server.
**Conversion path:** Hero → Problem/Solution (data blind spot) → How It Works → Sign up as early tester

**Why this is hot right now:** People are literally buying Mac Minis to run autonomous AI agents locally (OpenClaw, Moltbot). They want local data access but care about safety and control. LocalPush is the missing piece — safe, transparent, guaranteed delivery of local Mac data to their self-hosted automation server. The positioning: "More than a single claw — run an entire agent team on your own n8n, fed by your own data."

### Scenario F: "I just want my data in a spreadsheet"

**Source:** Word of mouth, "Did you know?" section going viral, less technical users seeing a friend's setup
**Content example:** Tweet showing a Google Sheet automatically filling up with Apple Podcasts listening history, or Claude Code token spend tracking.
**Visitor mindset:** "I don't have n8n or Zapier but I want my data somewhere I can see it."
**What they need from the page:** Google Sheets mentioned as a target, simple setup, no coding required.
**Conversion path:** Hero (sees Google Sheets mentioned) → How It Works → Sign up

**Why this matters:** Google Sheets as a target removes the "I need to be technical" barrier entirely. This opens LocalPush to a much broader audience — anyone who's curious about their own Mac data. The value stacking still works: they come for spreadsheet access, they discover the depth of hidden Mac data.

---

## Page Structure

### Section 1: Hero

**Layout:** Full-width dark section. Headline left, hero visual right (or centered with visual below).

**Headline:**
> Unlock your Mac data.

**Subheadline:**
> Your Mac stores incredible data — Claude Code usage, Apple Podcasts, Notes, Photos — locked away from your automation stack. LocalPush watches it and pushes changes to n8n, Make, Zapier, or even a Google Sheet with guaranteed delivery. Currently in beta.

**Primary CTA:**
```
[Become an Early Tester]     (button, primary blue → social auth modal)
```
Sign in with: **GitHub** · **Discord** · **Google**
Upon signup: user gets Homebrew tap command + direct DMG download link.

**Secondary CTA:**
```
[Star on GitHub →]  (subtle link — follow the project, get notified of OSS launch)
```

**Hero visual:** Screenshot or short looping video/GIF of:
- Menu bar icon (green) → click → tray popup showing healthy sources
- Or: the transparency preview screen showing real Claude Code data

**Trust strip below hero:**
```
Open Source (MIT)  ·  Guaranteed Delivery  ·  See Your Data Before It's Sent  ·  macOS
```

**Beta badge:** Small pill/badge near the headline or CTA: "Beta — OSS launch coming soon"

**Design notes:**
- Dark background (#1a1a1a or similar — match the app's visual language)
- The trust strip uses the traffic light visual language from the app (green dots)
- Social auth buttons should show provider icons (GitHub octocat, Discord logo, Google G) — feels like joining, not subscribing
- "Become an Early Tester" is more compelling than "Download" for beta phase — implies exclusivity and feedback loop

---

### Section 2: Problem → Solution

**Layout:** Two columns or before/after.

**The problem (left/before):**

> **Your Mac is full of data you can't reach.**
>
> Claude Code token stats. Apple Podcasts history. Notes. Photos metadata. It's all there, locked on your machine. Meanwhile, people are buying Mac Minis to run AI agents — but even they can't easily feed local data into their automation stacks. You're left with fragile cron jobs that fail silently, or manual exports that break your flow.

**The solution (right/after):**

> **LocalPush unlocks it.**
>
> A menu bar app that watches your local data and pushes changes to your automation server — n8n, Make, Zapier — or straight to a Google Sheet. Event-driven, not polling. Crash-safe, not "fingers crossed." You see your real data before anything is sent. Feed your AI agents with local data you control.

**Design notes:**
- "Can't reach" framing catches Scenario E visitors (local AI infra) alongside Scenario B (reliability)
- The OpenClaw/Moltbot reference is subtle ("buying Mac Minis to run AI agents") — don't name competitors, just reference the movement
- Keep it tight — two short paragraphs, not a wall of text
- Possible visual: locked data icons on the left → flowing data arrows on the right

---

### Section 3: How It Works

**Layout:** Four-step horizontal flow with icons/illustrations.

```
[1. Watch]           [2. Preview]          [3. Connect]          [4. Your Cadence]
LocalPush monitors   You see YOUR real     Connect your n8n,     Real-time, daily
your local data      data before anything  Zapier, or Make       digest, or weekly
sources for changes. is sent. Always.      receiver.             digest — you choose.

   📁 →                 👁️ →                  🔗 →                  ⏱️
```

**Step 1 — Watch:**
> Sources: Claude Code Stats, Apple Podcasts, Apple Notes, Apple Photos, and growing. LocalPush detects changes the moment they happen.

**Step 2 — Preview (Radical Transparency):**
> Before enabling any source, you see YOUR actual data — not samples, not descriptions. Your real token counts, your real podcast history. Nothing is ever sent without your inspection.

**Step 3 — Connect & Deliver (Guaranteed):**
> Connect your n8n, Zapier, or Make receiver — or just a Google Sheet — and have data sent securely with guaranteed delivery. WAL-backed queue survives app crashes, system reboots, and network outages. Every change is logged, retried, and confirmed. Zero data loss.

**Step 4 — Your Cadence:**
> Push in real-time, daily digest, or weekly digest — you choose per source. Get every change as it happens, or a clean summary when you want it.

**Design notes:**
- This is the core education section
- Step 2 is the differentiator — emphasize with a screenshot of the transparency preview
- Step 3 names the receivers explicitly — visitors need to see their platform mentioned
- Show n8n/Make/Zapier logos inline with step 3
- Consider a subtle animation (react-bits) as each step enters viewport

---

### Section 4: Use Cases (Value Stacking Section)

**Strategy: Value Stacking**

This section is the conversion accelerator. Every visitor arrives for ONE primary hook — but this section reveals the other use cases they didn't expect. The "DAMN, it got that too?!" reaction is what pushes them past the conversion threshold.

| Visitor came for... | Stacked surprise | Reaction |
|---------------------|-----------------|----------|
| Claude token tracking | Apple Podcasts, Notes, Photos sources | "It also unlocks my Apple data?!" |
| Apple data unlock | Claude Code stats source | "Wait, it tracks my Claude spend too?!" |
| n8n reliable triggers | Both Claude AND Apple sources | "This covers everything I need" |
| Local AI agent infra | All of the above + guaranteed delivery | "This is the missing piece" |

**Design implication:** ALL use cases must be visible without scrolling past the section. Don't tab or hide — display all cards at once so the stacking discovery happens visually.

**Layout:** Cards grid, all visible. 3+2 asymmetric or 2x3.

**Use Case 1: Claude Code Token Tracking**
> Track your AI spend automatically. LocalPush pushes Claude Code session stats to your n8n dashboard. See token usage, cost trends, and session patterns — without manual exports.
> **Setup time: 5 minutes.**

**Use Case 2: Apple Data Automation**
> Your Podcasts listening history, Notes changes, and Photos metadata — flowing into your automation stack. Build workflows triggered by your real life, not just your APIs.

**Use Case 3: File → Webhook Bridge**
> Watch any file or directory. When it changes, the new data hits your webhook. Replace fragile cron jobs with event-driven, guaranteed delivery.

**Use Case 4: Local AI Agent Infrastructure**
> Running self-hosted AI agents? Feed them local Mac data safely. LocalPush delivers to your own n8n server with radical transparency — you see exactly what's sent before it leaves your machine. More than a single claw — power an entire agent team with your own data.

**Use Case 5: Privacy-First Data Pipeline**
> Open source (MIT). Runs locally. No cloud dependency. You see exactly what's sent before it's sent. Audit the code yourself.

**Design notes:**
- Use Case 1 catches Scenario D visitors (Claude tracking)
- Use Case 4 catches Scenario E visitors (OpenClaw/local AI infra crowd)
- Use Case 5 catches Scenario C visitors (privacy/transparency)
- Layout: 2x3 grid or 3+2 asymmetric — 5 use cases is fine if cards are tight
- Each card should have a small icon and a "Learn more →" link to a blog post (future content)

---

### Section 5: "Did You Know?" (Value Discovery)

**Strategy:** Make the hidden Mac data tangible. Each card reveals something surprising about what's already on the visitor's machine — creating the visceral "I want to see MY data on that!" reaction. This section is the value stacking engine running at full throttle.

**Tone:** Fun, irreverent, breaks from the technical credibility sections above. This is the personality moment. Shareable, screenshot-able.

**Layout:** Scrolling cards or carousel. Each card: bold "Did you know?" header, the surprising fact, then the punchline.

---

**Card 1: Apple Podcasts**
> **Did you know?**
> Apple Podcasts stores your complete listening history — every episode, when you played it, how far you got — and full transcripts of everything you listened to. It's all sitting in a SQLite database on your Mac right now.
>
> **LocalPush it.**

---

**Card 2: Claude Code**
> **Did you know?**
> Claude Code stores every session name, the project you worked on, which git branch, timestamps, message counts, and your daily token spend broken down by model. Your entire AI work history lives in `~/.claude/`.
>
> **LocalPush it.**

---

**Card 3: Apple Notes**
> **Did you know?**
> Every note you've written — when you created it, when you last touched it, which folder it's in — is tracked in a database on your Mac. Your note-taking patterns tell a story.
>
> **LocalPush it.**

---

**Card 4: Apple Photos**
> **Did you know?**
> Your Photos library tracks metadata on every image — when, where, what camera, what's in the photo. Thousands of data points about your visual life, sitting in a SQLite database right now.
>
> **LocalPush it.**

---

**Card 5: What's Next? (Teaser)**
> **Did you know?**
> Your Mac knows your browsing history, screen time, calendar patterns, music taste, and more. We're unlocking new sources every release.
>
> **What would YOU LocalPush?**
> *(links to feedback/request form or Discord)*

---

**Design notes:**
- "LocalPush it." is the repeating punchline — becomes the catchphrase
- Each card should feel like a revelation, not a feature list
- Consider alternating background tints or card colors to keep the scroll engaging
- The last card ("What would YOU LocalPush?") turns the section into a feedback loop — visitors tell us what sources to build next, which directly feeds the value stacking growth lever
- Tone shift: the rest of the page is confident and technical; this section is playful and curious. It's the human moment.

---

### Section 6: Trust & Proof

**Layout:** Centered section with proof points.

| Signal | Content |
|--------|---------|
| **Open Source** | MIT licensed. Full source on GitHub. `[View on GitHub →]` |
| **Guaranteed Delivery** | WAL-backed queue. Survives crashes, reboots, outages. Zero data loss. |
| **Radical Transparency** | See your real data before anything is sent. Not just a checkbox — a core feature. |
| **Works With** | n8n · Make · Zapier · Google Sheets · ntfy · Any webhook endpoint |

**Design notes:**
- This section is for visitors who scrolled past the hero but aren't yet convinced
- "Works With" row should show logos/icons of n8n, Make, Zapier
- Consider GitHub star count badge when repo goes public

---

### Section 6: Early Access + GitHub

**Layout:** Centered CTA section, strong contrast.

**Headline:**
> Join the beta.

**Subheadline:**
> LocalPush is in testing and launches as open source soon. Sign up now to get immediate access and help shape what ships.

**Signup Flow (4 steps):**

```
[1. CTA Click]          [2. Intent Capture]       [3. Auth]              [4. Install Page]
"Become an Early    →   "What are you most    →   Sign in with       →   Welcome! Here's
 Tester" button         excited to try?"          GitHub / Discord       your install.
                        (single select)           / Google
```

**Step 1 — CTA Click:**
User clicks "Become an Early Tester" from hero or bottom CTA section. Opens modal or navigates to signup page.

**Step 2 — Intent Capture (pre-auth):**
One question, single-select. Captures what brought them here before they authenticate.

> **What are you most excited to try?**
>
> - Track my Claude Code token spend
> - Unlock my Apple data (Podcasts, Notes, Photos)
> - Replace my cron jobs with guaranteed delivery
> - Feed my self-hosted AI agents with local data
> - Push Mac data to a Google Sheet
> - Something else → (free text input)

**Why before auth, not after:** Completion rate drops after auth (they got what they wanted — the install). Asking before auth captures intent from everyone who starts the flow, even if they abandon. Tiny friction, high-value data.

**Step 3 — Social Auth:**
```
[Sign in with GitHub]    (button with GitHub octocat icon)
[Sign in with Discord]   (button with Discord icon)
[Sign in with Google]    (button with Google G icon)
```

**Step 4 — Install Page (post-auth):**
"Welcome, early tester" page with:

> **You're in.** Here's how to install LocalPush:
>
> **Option A: Homebrew (recommended)**
> ```
> brew tap madshn/localpush && brew install --cask localpush
> ```
> *(click to copy)*
>
> **Option B: Direct download**
> [Download DMG →]
>
> **What's next:**
> - Open LocalPush from your menu bar
> - Connect your n8n/Make/Zapier target (or Google Sheet)
> - Enable your first source and see your data
>
> [Star on GitHub →]  ·  [Join Discord →]

**Secondary CTA — GitHub follow (for non-signers):**
> **Prefer to wait for the open source launch?**
> [Follow on GitHub →]  (star/watch the repo when it goes public)

**Design notes:**
- Intent capture is ONE question, not a form. Takes 2 seconds. Zero typing required (unless "something else").
- Intent data is stored in Supabase alongside the user record — Rex can segment leads by intent, Leah can see which hooks drive signups
- GitHub sign-in is the default/top button — this is a developer tool, GitHub is home
- Discord second — signals community, younger dev crowd
- Google third — universal fallback
- Install page is the payoff — fast, clear, two options, no decision fatigue
- "Star on GitHub" and "Join Discord" on the install page capture additional engagement while momentum is high

**Data captured per signup:**

| Field | Source | Value |
|-------|--------|-------|
| Provider | Auth step | GitHub / Discord / Google |
| Profile | Auth step | Username, avatar, email (provider-dependent) |
| Intent | Pre-auth form | Which use case excited them |
| Signup date | Automatic | Timestamp |
| Installed? | Future — app phones home on first launch (optional) | Conversion tracking |

---

### Section 7: Blog / Content (Below Fold)

**Layout:** 2-3 card grid linking to blog posts.

**Initial posts (Leah produces separately):**
1. "Track Your Claude Code Token Spend in 5 Minutes" — Tutorial, Scenario D (highest engagement potential)
2. "Unlock Your Mac's Hidden Data for Your Automation Stack" — Discovery piece, Scenario A + E
3. "Beyond OpenClaw: Building a Local AI Agent Team with Your Own Data" — Thought piece, Scenario E (rides the hype wave)
4. "Radical Transparency: Why You Should See Your Data Before It's Sent" — Trust piece, Scenario C

**Design notes:**
- Blog lives at localpush.dev/blog
- Cards show: title, 1-line summary, read time
- Drives SEO long-tail traffic as content compounds

---

### Footer

- "Built by Right Aim" with link to rightaim.ai (portfolio backlink)
- GitHub link
- "LocalPush is open source (MIT)"

---

## Visual Design Direction

### Mood

Developer tool, not corporate SaaS. Think: Raycast website, Linear marketing, Warp terminal. Dark, clean, technical credibility with warmth.

### Color System

Inherit from the app's visual language (UX constitution):

| Token | Value | Usage on Landing Page |
|-------|-------|-----------------------|
| Background | `#0a0a0a` to `#1a1a1a` | Page background (slightly darker than app) |
| Card surface | `#1a1a1a` to `#2a2a2a` | Feature cards, use case cards |
| Text primary | `#ffffff` | Headlines, body |
| Text secondary | `#a0a0a0` | Descriptions, captions |
| Accent | `#4f9eff` | CTAs, links, interactive elements |
| Success | `#4ade80` | Trust signals, "guaranteed" indicators |
| Warning | `#fbbf24` | Subtle use only |
| Error | `#f87171` | Problem section (cron job pain) |

### Typography

- System font stack (-apple-system) for body — fast, native feel
- Monospace (SF Mono / JetBrains Mono) for install commands and code
- Large, confident headlines — not shouty, but clear

### Components (Bob's Library)

| Section | Suggested Components |
|---------|---------------------|
| Hero | Launch UI hero section |
| Problem/Solution | Launch UI feature comparison or custom two-column |
| How It Works | Launch UI features section (3-column) |
| Use Cases | shadcn cards with custom content |
| Trust | Launch UI social proof / badges section |
| CTA | Launch UI CTA section |
| Blog | shadcn cards grid |
| Footer | Launch UI footer |
| Animations | react-bits (viewport-triggered) or Magic UI (landing specific) |

### Key Visual Assets Needed

| Asset | Purpose | Tool |
|-------|---------|------|
| Hero screenshot/video | Show the app in action | Screen recording + mcp-image for polish |
| Transparency preview screenshot | Prove "see your data" claim | Screen capture from real app |
| Traffic light icon set | Visual language consistency | mcp-image or SVG |
| "Works With" logos | n8n, Make, Zapier, ntfy | SVG/PNG from official sources |
| Step illustrations (Watch/Preview/Deliver) | How-it-works flow | mcp-image |

---

## Stitch Prompt (Visual Exploration)

Based on UX constitution + landing page structure. Paste into Google Stitch for visual direction before Bob builds.

**Prompt quality notes** (from Bob's Stitch learning):
- Concrete visual elements, not abstract principles
- Spatial relationships ("up top," "below," "to the right")
- Interaction flow described sequentially
- Clear end state per section
- Screenshots > words for refinement iterations

### Full Prompt (paste-ready)

```
Design a single-page dark-themed landing page for LocalPush — a macOS menu bar app that unlocks hidden data on your Mac (Claude Code token stats, Apple Podcasts listening history, Apple Notes, Apple Photos metadata) and pushes changes to automation platforms or Google Sheets with guaranteed delivery. The product is in beta. Developer/power-user audience. Think Raycast, Linear, or Warp marketing pages — not corporate SaaS.

Dark mode only. Background #0a0a0a. Cards and surfaces #1a1a1a to #2a2a2a. White text on dark. Blue accent #4f9eff for all buttons and links. Green #4ade80 for trust signals. Monospace font for any code or terminal commands. System sans-serif for everything else. No gradients or glass effects — flat, clean, confident.

---

HERO — full width, takes the entire viewport height.

Left side: large bold headline "Unlock your Mac data." in white. Below it, a small rounded pill badge with "Beta" in a muted style, not loud but clearly visible. Below the badge, two lines of subheadline in grey (#a0a0a0): "Your Mac stores incredible data — Claude Code usage, Apple Podcasts, Notes, Photos — locked away from your tools. LocalPush watches it and pushes changes to n8n, Make, Zapier, or a Google Sheet. Guaranteed delivery."

Below the subheadline, a prominent blue (#4f9eff) button: "Become an Early Tester". Next to it or below it, a subtle text link: "Star on GitHub →" in the same blue but smaller, no background.

Right side: a large screenshot of a macOS menu bar tray popup. The popup has a dark background matching the page, showing a list of data sources (Claude Code Stats, Apple Podcasts, Apple Notes, Apple Photos) each with a small green dot to its left indicating healthy status, and a timestamp like "Last delivery: 2 min ago" in grey next to each. The popup looks native to macOS — rounded corners, slight shadow, compact.

Below the hero content, spanning full width, a horizontal trust strip with four items separated by centered dots: "Open Source (MIT) · Guaranteed Delivery · See Your Data First · macOS" — all in grey text, small, understated.

---

PROBLEM/SOLUTION — two columns side by side, slightly darker background (#0d0d0d) to create section separation.

Left column has a red-tinted (#f87171) heading: "Your Mac is full of data you can't reach." Below it, two short paragraphs in grey explaining that Claude Code stats, Apple Podcasts history, Notes, and Photos metadata are locked on your machine. Mentions that people are even buying Mac Minis to run AI agents but still can't feed them local data safely. Fragile cron jobs that fail silently. Manual exports that break flow.

Right column has a green-tinted (#4ade80) heading: "LocalPush unlocks it." Below it, two short paragraphs explaining the solution: a menu bar app that watches local data and pushes changes to n8n, Make, Zapier, or Google Sheets. Event-driven, crash-safe, transparent. You see your real data before anything leaves your machine.

Between the columns, or as a visual separator, a subtle arrow or flow indicator pointing from left (problem) to right (solution).

---

HOW IT WORKS — four equal columns in a horizontal row, each with a number and label at top, a small icon or illustration in the middle, and a short description below.

Column 1: "1. Watch" — icon of an eye or file — "LocalPush monitors your local data sources for changes the moment they happen. Sources: Claude Code, Apple Podcasts, Notes, Photos, and growing."

Column 2: "2. Preview" — icon of a magnifying glass on data — "Before anything is sent, you see YOUR real data. Not samples. Not descriptions. Your actual token counts, your real podcast history." This column should feel slightly emphasized (maybe a subtle border or glow) because it's the differentiator.

Column 3: "3. Connect" — icon of a plug or chain link — "Connect your n8n, Zapier, Make, or Google Sheets receiver. Data delivered securely with zero data loss. WAL-backed queue survives crashes, reboots, and outages." Below the description, show small platform logos in a row: n8n, Make, Zapier, Google Sheets — small, greyscale, recognizable.

Column 4: "4. Your Cadence" — icon of a clock or calendar — "Push in real-time, daily digest, or weekly digest. You choose per source." Below the text, show three small toggle-style labels: "Real-time" (highlighted), "Daily", "Weekly" — suggesting the user picks their preference.

---

USE CASES — section heading "What will you unlock?" centered. Below it, a grid of five cards arranged in a 3+2 pattern (three on top row, two centered below). All five cards visible at once without scrolling past this section — this is critical.

Each card: dark surface (#1a1a1a), rounded corners, small colored icon at top-left, bold white title, two-line grey description, and a subtle "Learn more →" link at bottom.

Card 1: "Claude Code Tracking" — blue icon — "Track your AI token spend automatically. Session stats flow to your dashboard. Setup: 5 minutes."

Card 2: "Apple Data Automation" — green icon — "Podcasts listening history, Notes changes, Photos metadata — flowing into your automation stack."

Card 3: "File → Webhook" — orange icon — "Watch any file. When it changes, the data hits your webhook. Event-driven, guaranteed delivery."

Card 4: "Local AI Agent Infrastructure" — purple icon — "Running self-hosted AI agents? Feed them local Mac data safely. More than a single claw — power an entire team."

Card 5: "Privacy-First Pipeline" — grey/white icon — "Open source. Runs locally. No cloud dependency. Audit the code yourself."

---

DID YOU KNOW — section with a playful heading "Did you know?" in a slightly larger, more casual weight. Below it, a horizontal scrolling row of cards (or a stacked carousel) with a different visual treatment than the use cases — slightly lighter backgrounds (#2a2a2a), maybe a thin colored left border per card.

Card 1 (left border green): "Did you know? Apple Podcasts stores your complete listening history — every episode, when you played it, how far you got — and full transcripts. It's all in a SQLite database on your Mac right now. LocalPush it."

Card 2 (left border blue): "Did you know? Claude Code stores every session name, project, git branch, timestamps, message counts, and daily token spend by model. Your entire AI work history lives in ~/.claude/. LocalPush it."

Card 3 (left border yellow): "Did you know? Every note you've written — when you created it, last touched, which folder — is tracked in a database on your Mac. Your note-taking patterns tell a story. LocalPush it."

Card 4 (left border pink): "Did you know? Your Photos library tracks metadata on every image — when, where, what camera, what's in the photo. Thousands of data points about your visual life. LocalPush it."

Card 5 (left border white, dashed): "Did you know? Your Mac knows your browsing history, screen time, calendar patterns, music taste, and more. We're unlocking new sources every release. What would YOU LocalPush?"

Each card's punchline ("LocalPush it." or "What would YOU LocalPush?") is in the blue accent color, bold.

---

TRUST — centered section, clean. Three icon-badge combos in a row: a shield icon with "Open Source (MIT)", a checkmark icon with "Guaranteed Delivery", an eye icon with "Radical Transparency". Below them, a "Works With" row showing small platform logos side by side: n8n, Make, Zapier, Google Sheets, ntfy — in greyscale with subtle hover color.

---

EARLY ACCESS CTA — full width section with a slightly brighter background (#141414) to stand out. Centered layout.

Large heading: "Join the beta." Below it: "LocalPush is in testing and launches as open source soon. Sign up now to get immediate access."

Below the text, the blue "Become an Early Tester" button, same as hero. When clicked, it opens a modal:

The modal (centered overlay, dark #1a1a1a, rounded) shows:
- Top: "What are you most excited to try?" as a question
- Below: six radio-button options in a vertical list — "Track my Claude Code token spend", "Unlock my Apple data", "Replace my cron jobs with guaranteed delivery", "Feed my self-hosted AI agents", "Push Mac data to a Google Sheet", "Something else" (with a small text input that appears)
- Below the options: three social auth buttons stacked vertically — "Sign in with GitHub" (dark button with octocat icon), "Sign in with Discord" (blurple button with Discord icon), "Sign in with Google" (white button with G icon)

After auth, the modal transitions to an install page: heading "You're in." with Homebrew install command in a monospace code block (with a copy button), an "or download DMG" link, and two small links: "Star on GitHub →" and "Join Discord →"

Below the main CTA button (outside the modal), a subtle text line: "Prefer to wait for the open source launch? Follow on GitHub →"

---

BLOG — three cards in a row, each showing a blog post title in white, a one-line summary in grey, and a read time. Dark card surfaces. Titles: "Track Your Claude Code Token Spend in 5 Minutes", "Unlock Your Mac's Hidden Data", "Beyond OpenClaw: Building a Local AI Agent Team"

---

FOOTER — dark, minimal. Left: "Built by Right Aim" as a subtle link. Center: GitHub icon link. Right: "Open Source (MIT)". Very compact, one line.

---

Overall feel: the page should feel like opening a well-made developer tool's website at 2am — dark, focused, zero noise, confident. Every section earns the next scroll. The "Did you know?" section is the personality break — slightly warmer, more curious, shareable. The rest is precise and technical. No stock photos, no generic illustrations. Screenshots of real UI, platform logos, monospace code blocks.
```

---

## Infrastructure Requirements (for Mira)

| Requirement | Solution |
|-------------|----------|
| Domain | localpush.dev (Mira to acquire + DNS) |
| Frontend | Render static site |
| Backend | Supabase (social auth + site-user table) |
| Auth providers | GitHub, Discord, Google (Supabase OOTB) |
| Post-auth page | "Welcome, early tester" with install instructions |
| Blog | Static pages at /blog (part of same deploy) |
| Analytics | PostHog (when wired) |
| SSL | Automatic (Render) |

**Supabase auth config:**
- Enable GitHub, Discord, Google providers
- site-user table captures: provider, profile data, signup date
- Rex reads this table for lead qualification
- Post-auth redirect → onboarding page with Homebrew command + DMG link

---

## Handoff Checklist

| Step | Owner | Artifact | Status |
|------|-------|----------|--------|
| 1. Experience brief | Leah | This document | **Done** |
| 2. Visual exploration | Leah/Bob | Stitch prototypes in `stitch-prototypes/web/` | **Done** |
| 3. Domain acquisition | Mira | localpush.dev | Not started |
| 4. Site build | Bob | React + Launch UI + shadcn | Not started |
| 5. Supabase setup | Mira | Auth + site-user table | Not started |
| 6. Deploy | Mira | Render static site | Not started |
| 7. Lead pickup | Rex | Read site-user table | Not started |
| 8. Blog content | Leah | 3 initial posts | Not started |
| 9. TOFU content | Leah | Demo video, Reddit/X posts, OpenClaw angle | Not started |
| 10. Post-auth onboarding page | Bob | Welcome + install instructions | Not started |

---

## Success Criteria

The page is working when:
- [ ] A visitor from any TOFU scenario (A-E) can find what they need within 10 seconds
- [ ] Early tester signup is < 2 clicks from hero (click CTA → pick provider → done)
- [ ] Social auth works for all 3 providers (GitHub, Discord, Google)
- [ ] Post-auth page delivers install instructions (Homebrew + DMG)
- [ ] Rex can read the site-user table and see provider + profile data
- [ ] GitHub follow link works (repo page or pre-launch "watch" setup)
- [ ] Blog has at least 1 post live
- [ ] "brew install" command is copy-pasteable on post-auth page
- [ ] Page loads in < 2 seconds
- [ ] Mobile-responsive (developers check phones too)

## Strategic Note: Value Stacking as Growth Lever

Each new source connector LocalPush ships is simultaneously:
1. **A new TOFU hook** — a new audience segment has a reason to visit
2. **A stacking bonus** — every existing visitor gets another "DAMN, it got that too?!" moment
3. **A conversion multiplier** — more reasons to sign up, more reasons to stay

This means the product roadmap (which sources to build next) directly influences the growth ceiling. When evaluating new sources, ask: "Does this unlock a new audience segment we can't currently reach?" If yes, it's a growth lever, not just a feature.

**Implication for persona-fishing:** Growth is bounded by the number of distinct audiences we can attract. Each source = one potential primary persona. The landing page scales with the product.

---

## Metrics to Track

| Metric | Definition | Target (30-day) |
|--------|-----------|-----------------|
| **Early tester signups** | Social auth completions | 50 (stretch: 150) |
| **Signup by provider** | GitHub vs Discord vs Google split | Track for channel insight |
| **GitHub follows/stars** | Repo stars + watchers | 100 (stretch: 300) |
| **Page visitors** | Unique visitors to localpush.dev | 500 (stretch: 1,500) |
| **Signup conversion rate** | Visitors → signups | 10% target |
| **Post-auth install rate** | Signups → Homebrew/DMG download | Track (optimize later) |
| **Blog reads** | Blog post page views | Track per post |
