# CCM Alfred Workflow

Search and launch Claude Code projects via CCM's HTTP API.

## Install

### Option 1: Link (for development)

```bash
ln -s "$(pwd)" ~/Library/Application\ Support/Alfred/Alfred.alfredpreferences/workflows/ccm
```

### Option 2: Export as .alfredworkflow

```bash
cd alfred-workflow
zip -r ../CCM.alfredworkflow . -x "README.md" -x ".DS_Store"
# Double-click CCM.alfredworkflow to import
```

## Setup

1. Open Alfred Preferences → Workflows → CCM
2. Click **[x]** (Environment Variables) in the top-right
3. Set `CCM_API_TOKEN` to the token generated in CCM Settings → HTTP API
4. Optionally change `CCM_API_PORT` (default: `23890`)

## Usage

Type `ccm` in Alfred, then:

| Action | Key | Description |
|--------|-----|-------------|
| Launch Claude Code | `Enter` | Opens Claude in the selected project's terminal |
| Copy path | `Cmd+Enter` | Copies project path to clipboard |
| Open in Finder | `Alt+Enter` | Opens project directory in Finder |

Search is real-time — just keep typing after `ccm` to filter projects.
