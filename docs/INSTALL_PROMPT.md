# Nellie-RS Install Prompt

**Instructions**: Download the [latest release](https://github.com/mmorris35/nellie-rs/releases), unzip it, open a terminal in that folder, start your AI agent (OpenClaw, Claude Code, etc.), and paste this prompt:

---

## The Prompt

```
Install Nellie-RS from this folder. Steps:

1. Detect my platform (macOS/Linux, x86_64/aarch64) and find the matching binary
2. Create ~/.nellie-rs/{logs,models} directories
3. Copy the binary to ~/.nellie-rs/nellie and make it executable
4. Create symlink at ~/.local/bin/nellie
5. Download the embedding model:
   curl -L -o ~/.nellie-rs/models/all-MiniLM-L6-v2.onnx \
     "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx"

6. Find my code directories - look for folders containing git repos in:
   - ~/code, ~/Code, ~/projects, ~/Projects, ~/dev, ~/Development
   - ~/Documents/code, ~/Documents/projects
   - Or ask me where my code lives

7. Create ~/.nellie-rs/config.toml with those watch_dirs

8. Set up the service:
   - macOS: create ~/Library/LaunchAgents/com.nellie-rs.plist and load it
   - Linux: create ~/.config/systemd/user/nellie.service and enable it

9. Verify with: curl http://localhost:8765/health

10. Add Nellie to your MCP config (mcporter add nellie --sse http://localhost:8765/sse)

After install, tell me what directories are being watched and confirm it's running.
```

---

## Shorter Version

```
Install Nellie-RS from this folder:
1. Find the binary for my platform, copy to ~/.nellie-rs/nellie
2. Download embedding model from HuggingFace (all-MiniLM-L6-v2.onnx)
3. Find my code directories (~/code, ~/projects, etc.) and configure watch_dirs
4. Set up launchd (macOS) or systemd (Linux) service
5. Verify it's running on localhost:8765
6. Add to MCP: mcporter add nellie --sse http://localhost:8765/sse
```

---

## What Gets Installed

```
~/.nellie-rs/
├── nellie                              # Binary
├── config.toml                         # Your config
├── nellie.db                           # Database (created on first run)
├── logs/
│   └── nellie.log
└── models/
    └── all-MiniLM-L6-v2.onnx          # Embedding model (~90MB)

~/.local/bin/nellie -> ~/.nellie-rs/nellie   # Symlink for PATH
```

**macOS**: `~/Library/LaunchAgents/com.nellie-rs.plist`
**Linux**: `~/.config/systemd/user/nellie.service`

---

## After Install

Your agent can now use Nellie:

```bash
# Search your code semantically
mcporter call nellie search_code --query "authentication flow"

# Save lessons
mcporter call nellie add_lesson --title "Pattern" --content "Always do X when Y"

# Check status
mcporter call nellie get_status
```

See [AGENT_GUIDE.md](./AGENT_GUIDE.md) for full tool reference.
