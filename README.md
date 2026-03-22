# CCM — Claude Config Manager

**A desktop app for managing Claude Code configurations across all your projects.**

![ui](images/ui.jpeg)

## The Problem

If you use [Claude Code](https://docs.anthropic.com/en/docs/claude-code) across multiple projects, you'll quickly run into these pain points:

- **Configuration scattered everywhere** — Skills, agents, rules, hooks, and commands are buried in each project's `.claude/` directory. There's no single place to see what you have or what's being used where.
- **No reuse** — You wrote a great skill in Project A. To use it in Project B, you manually copy files and hope they stay in sync. They won't.
- **No visibility** — Which projects have Claude configured? What resources exist globally? What MCP servers are running? You have to dig through filesystems to find out.
- **Tedious setup** — Every new project means recreating environment variables, copying over your favorite rules, and reconfiguring permissions. Again.
- **No sharing** — Teams have no mechanism to share Claude configurations. Each developer maintains their own set of skills and rules independently.

## The Solution

CCM gives you a **central dashboard** for everything Claude Code — across all your projects, with a shared library, registry support, and cross-project resource management.

**See everything at a glance.** One dashboard shows all your projects, resources, plugins, and MCP servers. Search across everything instantly.

**Write once, use everywhere.** Store resources in a central library (`~/.claude-manager/library/`), then install them into any project via symlink or copy. Update the library version, and all symlinked projects get the change automatically.

**Share with your team.** Git-based registries let teams publish, version, and distribute Claude configurations — skills, agents, rules, and complete plugin packs.

**Automate with APIs.** A local HTTP API lets you integrate CCM with Raycast, Alfred, shell scripts, or any tool that speaks HTTP.

---

## Features

### Project Management

- **Auto-discovery** — Scans your filesystem and `~/.claude/projects.json` to find all Claude-configured projects
- **Language detection** — Identifies project language (Go, Rust, TypeScript, Python, Java, etc.) from build files
- **Quick Launch** — Open Claude Code in any project with one click; injects environment variables and tracks launch history
- **Pin favorites** — Pin frequently used projects for quick access in the command palette
- **Terminal choice** — Launch in Terminal.app, iTerm2, or Warp

### Resource Management

Manage six types of Claude Code resources:

| Type | Description |
|------|-------------|
| **Skills** | Reusable instruction blocks and domain knowledge |
| **Agents** | AI agent definitions and personas |
| **Rules** | Behavioral guidelines for Claude |
| **Hooks** | Event hooks (JSON configuration) |
| **Commands** | Custom slash commands |
| **MCP Servers** | Model Context Protocol server configurations |

Each resource can exist in multiple scopes:

- **Global** (`~/.claude/`) — Active for all projects
- **Project** (`project/.claude/`) — Specific to one project
- **Library** (`~/.claude-manager/library/`) — Central reusable storage
- **Registry** — Shared via git-based registries

### Central Library

- Store resources in `~/.claude-manager/library/` as your single source of truth
- **Install to Project** — Symlink or copy library resources into any project
- **Deploy to Global** — Push library resources to `~/.claude/` for universal access
- **Plugin Packs** — Bundle multiple resources into installable packages
- **Link health check** — Verify all symlinks are valid and targets exist

### Registry System

- **Git-based sharing** — Add registries by URL, sync via git pull/push
- **Marketplace** — Browse plugins and resources from team or community registries
- **Publish** — Share your resources and plugin packs to writable registries
- **Install** — Pull resources or plugins from registries into your library, projects, or global scope

### Plugin Management

- **Scan installed plugins** — Detect plugins in `~/.claude/plugins/`
- **Extract to library** — Pull individual resources out of plugins for reuse
- **Registry plugins** — Install external plugins from registries with MCP server support
- **Library plugin packs** — Create and manage your own plugin bundles

### MCP Server Management

- View MCP servers from `.mcp.json` (global and per-project)
- Display server type, command, args, URL, and environment variables
- Track MCP servers provided by plugins and registries

### Environment Variables

- **Global env vars** — Set variables passed to `claude` CLI on every launch
- **Project env vars** — Override or extend globals per project
- **Merged view** — See the final computed environment for each project
- Automatic injection when launching Claude in terminal

### Synchronization

- **Full sync** — Reconcile filesystem state with database across all scopes
- **File watcher** — Real-time change detection on `~/.claude/` and `~/.claude-manager/`
- **Content hashing** — Detect modifications by file hash, not just timestamps
- Six-stage sync with progress reporting

### External Tool Integration

CCM provides a local HTTP API that any tool can call — Raycast, Alfred, shell scripts, CI pipelines, or custom dashboards.

#### HTTP API

| Endpoint | Description |
|----------|-------------|
| `GET /api/health` | Health check (no auth required) |
| `GET /api/projects?q=keyword` | Search/list projects |
| `GET /api/projects/:id` | Project detail |
| `POST /api/projects/:id/launch` | Launch Claude Code in project |

- Bearer token authentication (SHA-256 hashed, constant-time verification)
- Token generation in Settings UI (shown once, copy to clipboard)
- Configurable port (default: 23890)
- Runtime enable/disable without restart

#### Raycast

Bundled Raycast Extension (`raycast-extension/`):

- **Search Projects** — Real-time project search from Raycast
- **Launch Claude Code** — Start Claude in any project with Enter
- **Copy Path** — Copy project path to clipboard
- Configurable API token and port in Raycast preferences

#### Alfred

The HTTP API can be called directly from Alfred Workflows using `curl`:

```bash
# Search projects
curl -s -H "Authorization: Bearer YOUR_TOKEN" \
  "http://127.0.0.1:23890/api/projects?q=myproject"

# Launch Claude Code in a project
curl -s -X POST -H "Authorization: Bearer YOUR_TOKEN" \
  "http://127.0.0.1:23890/api/projects/PROJECT_ID/launch"
```

Create a **Script Filter** in Alfred with the search endpoint, parse the JSON response to build Alfred result items, then use a **Run Script** action to call the launch endpoint.

#### Shell Scripts

```bash
# List all projects
curl -s -H "Authorization: Bearer $CCM_TOKEN" \
  http://127.0.0.1:23890/api/projects | jq '.data[].name'

# Launch a project by name
ID=$(curl -s -H "Authorization: Bearer $CCM_TOKEN" \
  "http://127.0.0.1:23890/api/projects?q=myapp" | jq -r '.data[0].id')
curl -s -X POST -H "Authorization: Bearer $CCM_TOKEN" \
  "http://127.0.0.1:23890/api/projects/$ID/launch"
```

### System Tray

- Application persists in system tray when window is closed
- Tray menu: Show Window, API status, Quit
- HTTP API remains accessible with window hidden

### Settings

- **Network Proxy** — HTTP/HTTPS or SOCKS5 proxy for registry sync (with connection test)
- **Command Palette** — Global hotkey (default: `Meta+K`) with customizable shortcut
- **Terminal** — Choose preferred terminal application
- **HTTP API** — Enable/disable, port, token management

---

## Tech Stack

| Layer | Technology |
|-------|------------|
| Framework | [Tauri 2.x](https://v2.tauri.app/) |
| Frontend | React 19 + TypeScript |
| Backend | Rust |
| UI | [shadcn/ui](https://ui.shadcn.com/) + Tailwind CSS v4 |
| State | Zustand v5 |
| Database | SQLite (via rusqlite) |
| HTTP API | axum |
| Routing | react-router-dom v7 |

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/) toolchain
- macOS (symlink features require Unix)

### Development

```bash
# Install dependencies
npm install

# Start development mode (Vite + Tauri)
npm run tauri dev

# Run frontend tests
npm test

# Run backend tests
cd src-tauri && cargo test
```

### Build

```bash
# Build for current architecture
npm run tauri build
```

#### macOS — Apple Silicon (arm64)

On an Apple Silicon Mac (M1/M2/M3/M4), `npm run tauri build` produces an arm64 binary by default.

To cross-compile for Intel from an Apple Silicon Mac:

```bash
rustup target add x86_64-apple-darwin
npm run tauri build -- --target x86_64-apple-darwin
```

#### macOS — Intel (x86_64)

On an Intel Mac, `npm run tauri build` produces an x86_64 binary by default.

To cross-compile for Apple Silicon from an Intel Mac:

```bash
rustup target add aarch64-apple-darwin
npm run tauri build -- --target aarch64-apple-darwin
```

#### macOS — Universal Binary

To build a universal binary (runs natively on both Intel and Apple Silicon):

```bash
rustup target add x86_64-apple-darwin aarch64-apple-darwin
npm run tauri build -- --target universal-apple-darwin
```

Build output is located in `src-tauri/target/release/bundle/` — includes `.app`, `.dmg`, and `.pkg` formats.

### Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| **macOS (arm64)** | Fully supported | Primary development platform |
| **macOS (x86_64)** | Fully supported | Cross-compile from arm64 or build natively |
| **Linux** | Partial | Builds and runs. Symlink features work. Terminal launch uses macOS-specific `osascript` — needs platform-specific implementation for Linux terminals (e.g. gnome-terminal, kitty). System tray depends on desktop environment support. |
| **Windows** | Not supported | Symlink management uses `#[cfg(unix)]` guards — Windows symlinks require elevated privileges and use different APIs. Terminal launch is macOS-only (`osascript`). Contributions welcome. |

### Raycast Extension

```bash
cd raycast-extension
npm install
npm run dev    # Loads extension into Raycast
```

## Data Storage

All data is stored locally:

| Path | Content |
|------|---------|
| `~/.claude-manager/ccm.db` | SQLite database (projects, resources, settings) |
| `~/.claude-manager/library/` | Central resource library |
| `~/.claude-manager/registries/` | Local clones of git registries |
| `~/.claude/` | Global Claude Code configuration (managed by CCM) |

## License

MIT
