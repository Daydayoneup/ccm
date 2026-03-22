# CCM Raycast Extension

Search and launch Claude Code projects via CCM's HTTP API.

## Install

```bash
cd raycast-extension
npm install
npm run dev    # Loads extension into Raycast
```

Requires [Raycast CLI](https://developers.raycast.com/) — in Raycast, search "Install CLI" to install.

## Setup

First launch will prompt for preferences:

- **API Token** (required) — Generate in CCM Settings → HTTP API → Generate Token
- **API Port** (optional) — Default `23890`, change if CCM uses a different port

## Usage

Open Raycast and type "Search Projects", then:

| Action | Key | Description |
|--------|-----|-------------|
| Launch Claude Code | `Enter` | Opens Claude in the selected project's terminal |
| Copy path | `Cmd+C` | Copies project path to clipboard |

Search is real-time — results update as you type.

Each project shows:
- Name (with pin icon if pinned)
- File path
- Language tag
- Launch count
