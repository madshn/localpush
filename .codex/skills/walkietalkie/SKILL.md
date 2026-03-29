---
description: "WalkieTalkie for Codex in localpush. Use when the user or Codex @mentions Bob or another associate."
---

# WalkieTalkie — Codex

Repo-local WalkieTalkie for Codex in `localpush`.

## Identity

In this repo, Codex is the Product Coordinator and speaks on the bus as `lpush`.

- Main contact: `@bob`
- Also available: `@mira`, `@aston`, `@leah`, `@rex`, `@metrick`
- Use this skill when the user asks you to contact an associate, or when you decide to consult one while working on this repo.

## Trigger

Activate this skill when the prompt contains an `@associate` mention, for example:

- `@bob can you sanity-check the factory flow?`
- `Ask @mira what the code signing requirement is`

Do not treat this as a slash command.

## Behavior

1. Treat Codex as caller `lpush` unless the user explicitly asks for a different sender.
2. Decide whether this is a direct follow-up to the immediately prior exchange with the same associate.
   - Follow-up examples: `tell bob thanks`, `ask mira to clarify that last point`, `what about X?` when `X` clearly refers to the prior answer.
   - New-question examples: anything that changes topic, asks for current or latest state, or is not clearly building on the prior answer.
   - Rule of thumb: when in doubt, start a new conversation.
3. Use the repo-local sender script:

```bash
.codex/skills/walkietalkie/scripts/walkietalkie.sh --caller lpush --message "$ARGUMENTS"
```

For explicit follow-ups, add `--follow-up`:

```bash
.codex/skills/walkietalkie/scripts/walkietalkie.sh --caller lpush --follow-up --message "$ARGUMENTS"
```

4. The script will:
   - register this local Codex session on the bus
   - enrich bare `@name` mentions only when the current git branch clearly points at an issue context
   - send via `post_team_message`
   - poll deliveries, reactions, and reply messages
   - print status updates to stderr while waiting
5. Parse the result:
   - first lines contain `CONV_ID=...`, `TARGET=...`, and `STATUS=...`
   - reply body appears after `---`
6. Relay the reply in this exact format:

```text
[bob] reply text here
```

Use lowercase name in square brackets, no bold, no colon.

## User-Facing Updates

Keep the terminal calm. Do not narrate internal setup, file exploration, script invocation, or background-process mechanics.

- Before running the sender, emit one short status line only:
  - New thread: `Message sent to Bob... New request, not continuing a previous conversation.`
  - Follow-up: `Message sent to Bob... Continuing the previous conversation.`
- During the wait, emit at most one short progress update when the worker is clearly processing:
  - `Bob is working on a reply...`
- Then wait silently for the final reply.
- Send exactly once per user prompt. Do not automatically retry or send a second fresh question unless the user explicitly asks.

Avoid messages like:
- `I'm pulling up the workflow first`
- `Explored 2 files`
- `Background terminal finished with ...`
- `I'm staying with the thread until it resolves`

## Timeout And Failure Handling

If the script returns `STATUS=timeout`, `STATUS=failed`, or `STATUS=error`:

- Do not answer the associate's question yourself
- Do not summarize your own repo knowledge as if it came from the associate
- Do not silently retry
- Relay the returned status text plainly instead

Only produce an associate-style final relay like `[bob] ...` when `STATUS=reply`.

## Notes

- The script stores server-assigned `conversation_id` values per associate in `.codex/state/walkietalkie-conversations.json`.
- Fresh thread is the default. Stored conversation IDs are reused only when `--follow-up` is passed.
- Bare `@name` mentions are only auto-enriched on issue branches. Repo-only context is too broad and can misroute unrelated questions.
- Tag-only posts (`#standup ...`) are fire-and-forget and return a short confirmation string after `---`.
- The current client poll cursor still uses client `SEND_TS`, matching the live Claude skill behavior.
- Repo-local Codex Stop reactions are wired through `.codex/hooks.json` and `.codex/reaction-hook.sh`.
