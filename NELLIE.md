# Nellie - Code Memory System

> **Drop this into any agent workspace so they know how to use Nellie.**

## What is Nellie?

Nellie is a **persistent code memory server** running on the mini-dev-server. It stores:
- **Lessons** — Things you've learned (bugs, patterns, gotchas, preferences)
- **Checkpoints** — Snapshots of your working state (for context recovery after compaction)
- **Code search** — Semantic search across indexed codebases

Think of it as your **experiential memory** that survives across sessions.

## Connection

- **Host:** `100.87.147.89` (Tailscale)
- **Port:** `8765`
- **Protocol:** HTTP POST to `/mcp/invoke`

## Quick Reference

### Check Status
```bash
curl -s -X POST http://100.87.147.89:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "get_status", "arguments": {}}'
```

### Check Your Agent Status
```bash
curl -s -X POST http://100.87.147.89:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "get_agent_status", "arguments": {"agent": "YOUR_AGENT_NAME"}}'
```

---

## Lessons

Lessons are persistent knowledge. When you learn something worth remembering, add it.

### Add a Lesson
```bash
curl -s -X POST http://100.87.147.89:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "name": "add_lesson",
    "arguments": {
      "title": "Short descriptive title",
      "content": "Detailed explanation of what you learned.",
      "tags": ["relevant", "tags"],
      "severity": "info"
    }
  }'
```

**Severity levels:** `info`, `warning`, `critical`

### Search Lessons
```bash
curl -s -X POST http://100.87.147.89:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "name": "search_lessons",
    "arguments": {"query": "your search terms"}
  }'
```

### List All Lessons
```bash
curl -s -X POST http://100.87.147.89:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "list_lessons", "arguments": {}}'
```

### When to Add Lessons
- Bug you figured out (especially non-obvious ones)
- API quirks or gotchas
- Architecture decisions and why
- Things that broke and how you fixed them
- Patterns that work well
- Mistakes to avoid
- Mike says "from now on..." or "always do X"
- Mike expresses a preference
- Any reusable knowledge

**Don't wait to be prompted!** If you learned something, persist it immediately.

---

## Checkpoints

Checkpoints save your working state. Use them for context recovery after session compaction.

### Save a Checkpoint
```bash
curl -s -X POST http://100.87.147.89:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "name": "add_checkpoint",
    "arguments": {
      "agent": "YOUR_AGENT_NAME",
      "working_on": "Brief description of current task",
      "state": {
        "decisions": ["Key decision 1", "Key decision 2"],
        "flags": ["IDLE"]
      }
    }
  }'
```

### Get Recent Checkpoints
```bash
curl -s -X POST http://100.87.147.89:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "name": "get_recent_checkpoints",
    "arguments": {"agent": "YOUR_AGENT_NAME", "limit": 3}
  }'
```

### When to Checkpoint
- Before complex multi-step work
- After meaningful progress
- During heartbeats (if activity since last checkpoint)
- Before you think compaction might happen

---

## Code Search

Search indexed codebases semantically.

```bash
curl -s -X POST http://100.87.147.89:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "name": "search_code",
    "arguments": {"query": "how does authentication work"}
  }'
```

---

## Using with mcporter (Alternative)

If mcporter is configured:
```bash
mcporter call 'http://100.87.147.89:8765/mcp.add_lesson({
  "title": "Example",
  "content": "Details here",
  "tags": ["example"],
  "severity": "info"
})' --allow-http
```

---

## Pro Tips

1. **Search before solving** — Someone might have already figured it out
2. **Be specific in titles** — "Telegram group ID format requires -100 prefix" > "Telegram issue"
3. **Include context** — Why did this matter? What was the symptom?
4. **Tag consistently** — Use lowercase, common terms
5. **Checkpoint after wins** — Future-you will thank present-you

---

## Example: Session Start Recovery

If you wake up with missing context:
```bash
# Check your status
curl -s -X POST http://100.87.147.89:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "get_agent_status", "arguments": {"agent": "YOUR_AGENT_NAME"}}'

# Get recent checkpoints
curl -s -X POST http://100.87.147.89:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "get_recent_checkpoints", "arguments": {"agent": "YOUR_AGENT_NAME", "limit": 3}}'

# Search for relevant lessons
curl -s -X POST http://100.87.147.89:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "search_lessons", "arguments": {"query": "topic you are working on"}}'
```

---

*Nellie is named after the elephant — because elephants never forget.*
