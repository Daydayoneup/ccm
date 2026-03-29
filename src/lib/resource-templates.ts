import type { ResourceType } from '@/types/v2';

export const resourceTemplates: Record<ResourceType, (name: string) => string> = {
  skill: (name) => `---\nname: ${name}\ndescription: \n---\n\n# ${name}\n\n`,
  agent: (name) => `# ${name} Agent\n\nYou are...\n`,
  rule: (name) => `# ${name}\n\n`,
  hook: (name) => `{\n  "event": "PreToolUse",\n  "matcher": "${name}",\n  "hook_config": {\n    "type": "command",\n    "command": "echo ${name}"\n  }\n}\n`,
  mcp_server: () => `{\n  "command": "",\n  "args": []\n}\n`,
  command: (name) => `# ${name}\n\nUsage: /${name}\n\n`,
};
