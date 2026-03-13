# julesctl

> Jules AI multi-session orchestrator — single and parallel session management, automatic patch apply, and conflict resolution.

## Modes

| Mode | Command | Description |
|------|---------|-------------|
| **Single** | `julesctl watch` | Watch one Jules session, auto-apply patches |
| **Orchestrated** | `julesctl orchestrate "<goal>"` | Jules manager breaks goal into tasks, opens worker sessions automatically |
| **Manual** | `julesctl watch` (mode=manual) | You define sessions, julesctl applies patches in queue order |

## Install

```bash
git clone https://github.com/rvoidex7/julesctl
cd julesctl
cargo install --path .
```

## Setup

```bash
julesctl init
# Edit ~/.config/julesctl/config.toml
```

Config example:

```toml
api_key = "YOUR_JULES_API_KEY"

# Mode 1 — Single session
[[repos]]
path              = "/home/user/projects/my-app"
display_name      = "My App"
mode              = "single"
post_pull         = "cargo build"
single_session_id = "14550388554331055113"

# Mode 2 — Orchestrated
[[repos]]
path         = "/home/user/projects/another-app"
display_name = "Another App"
mode         = "orchestrated"
post_pull    = ""

# Mode 3 — Manual multi-session
[[repos]]
path         = "/home/user/projects/big-app"
display_name = "Big App"
mode         = "manual"
post_pull    = ""
```

### Finding your session ID

Jules URL format: `https://jules.google/tasks/14550388554331055113`  
The numeric part is your `session_id`.

### API key

Get it from [https://aistudio.google.com/apikey](https://aistudio.google.com/apikey)

---

## Mode 1 — Single Session

```bash
cd ~/projects/my-app
julesctl watch                              # start watch loop
julesctl send "add a resign button"        # send prompt to Jules
julesctl status                            # show last 8 activities
julesctl status --count 20                 # show last 20
```

When Jules produces changes, julesctl:
1. Fetches patch from Jules artifacts API
2. Applies it to local `jules-orchestrator` branch via `git apply`
3. Commits and runs `post_pull` script

---

## Mode 2 — Orchestrated

```bash
cd ~/projects/another-app
julesctl orchestrate "Build a user auth system with login, register, and profile pages"
```

julesctl:
1. Creates a Jules manager session with a bootstrap prompt
2. Jules manager writes `.julesctl-tasks.json` with task breakdown
3. julesctl watches that file, opens worker sessions for each task
4. Patches from worker sessions are applied in FIFO order
5. Conflicts are sent back to manager session — Jules resolves them automatically

You never touch a file.

---

## Mode 3 — Manual Multi-Session

```bash
cd ~/projects/big-app

# Add sessions to queue
julesctl session add 14550388554331055113 "Login feature" --position 0
julesctl session add 99887766554433221100 "Profile feature" --position 1
julesctl session add 11223344556677889900 "Notifications" --position 2

julesctl session list       # verify queue
julesctl watch              # start applying in order
```

Conflicts are sent to the first session in the queue for resolution.

---

## TUI Dashboard and cli-chat-rs Integration

Running `julesctl` with no arguments launches the **Project Dashboard** TUI. This dashboard scopes directly to your currently active project directory and presents available tasks/sessions (Single, Orchestrated Manager, or Manual task queues).

From this Dashboard, you can select an active task. `julesctl` will then spin up the `cli-chat-rs` generic TUI specifically scoped to that single session. This ensures that `cli-chat-rs` remains a lightweight and decoupled chat framework utilizing `ratatui`, whilst `julesctl` manages orchestration and project context.

---

## How patches work

- All patches fetched via `jules.googleapis.com/v1alpha/sessions/{id}/artifacts`
- Applied via `git apply` to local `jules-orchestrator` branch
- Jules-owned remote branches are never checked out
- Conflicts detected via `git apply --check` before modifying any files
