import * as React from "react";
import { ChevronDownIcon, ChevronRightIcon, XIcon } from "lucide-react";

import type { SkillFrontmatter } from "@/types/v2";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";

interface SkillFrontmatterFormProps {
  value: SkillFrontmatter;
  onChange: (value: SkillFrontmatter) => void;
}

const MODEL_OPTIONS = [
  { value: "_default", label: "Default" },
  { value: "sonnet", label: "Sonnet" },
  { value: "opus", label: "Opus" },
  { value: "haiku", label: "Haiku" },
];

const EFFORT_OPTIONS = [
  { value: "_default", label: "Default" },
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
  { value: "max", label: "Max" },
];

export function SkillFrontmatterForm({
  value,
  onChange,
}: SkillFrontmatterFormProps) {
  const [advancedOpen, setAdvancedOpen] = React.useState(false);
  const [toolInput, setToolInput] = React.useState("");

  function update(partial: Partial<SkillFrontmatter>) {
    onChange({ ...value, ...partial });
  }

  function handleNameChange(e: React.ChangeEvent<HTMLInputElement>) {
    const sanitized = e.target.value
      .toLowerCase()
      .replace(/[^a-z0-9-]/g, "")
      .slice(0, 64);
    update({ name: sanitized || null });
  }

  function handleModelChange(selected: string) {
    update({ model: selected === "_default" ? null : selected });
  }

  function handleEffortChange(selected: string) {
    update({ effort: selected === "_default" ? null : selected });
  }

  function handleToolKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") {
      e.preventDefault();
      const tool = toolInput.trim();
      if (!tool) return;
      const current = value.allowed_tools ?? [];
      if (!current.includes(tool)) {
        update({ allowed_tools: [...current, tool] });
      }
      setToolInput("");
    }
  }

  function removeTool(tool: string) {
    const updated = (value.allowed_tools ?? []).filter((t) => t !== tool);
    update({ allowed_tools: updated.length > 0 ? updated : null });
  }

  const modelValue = value.model ?? "_default";
  const effortValue = value.effort ?? "_default";

  return (
    <div className="space-y-3">
      {/* Name */}
      <div className="space-y-1">
        <Label htmlFor="skill-name" className="text-[11px] uppercase tracking-wider text-muted-foreground">
          Name
        </Label>
        <Input
          id="skill-name"
          placeholder="my-skill-name"
          value={value.name ?? ""}
          onChange={handleNameChange}
          maxLength={64}
          className="h-8 text-sm"
        />
      </div>

      {/* Description */}
      <div className="space-y-1">
        <Label htmlFor="skill-description" className="text-[11px] uppercase tracking-wider text-muted-foreground">
          Description
        </Label>
        <Textarea
          id="skill-description"
          rows={2}
          placeholder="What this skill does…"
          value={value.description ?? ""}
          onChange={(e) => update({ description: e.target.value || null })}
          className="text-sm resize-none"
        />
      </div>

      {/* Model + Effort in one row */}
      <div className="grid grid-cols-2 gap-2">
        <div className="space-y-1">
          <Label htmlFor="skill-model" className="text-[11px] uppercase tracking-wider text-muted-foreground">
            Model
          </Label>
          <Select value={modelValue} onValueChange={handleModelChange}>
            <SelectTrigger id="skill-model" className="h-8 text-sm">
              <SelectValue placeholder="Default" />
            </SelectTrigger>
            <SelectContent>
              {MODEL_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="space-y-1">
          <Label htmlFor="skill-effort" className="text-[11px] uppercase tracking-wider text-muted-foreground">
            Effort
          </Label>
          <Select value={effortValue} onValueChange={handleEffortChange}>
            <SelectTrigger id="skill-effort" className="h-8 text-sm">
              <SelectValue placeholder="Default" />
            </SelectTrigger>
            <SelectContent>
              {EFFORT_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* Allowed Tools */}
      <div className="space-y-1">
        <Label htmlFor="skill-tools" className="text-[11px] uppercase tracking-wider text-muted-foreground">
          Allowed Tools
        </Label>
        <div className="flex flex-wrap gap-1 min-h-8 rounded-md border border-input bg-transparent px-2 py-1.5">
          {(value.allowed_tools ?? []).map((tool) => (
            <Badge
              key={tool}
              variant="secondary"
              className="flex items-center gap-0.5 pr-0.5 text-[11px] h-5"
            >
              {tool}
              <button
                type="button"
                onClick={() => removeTool(tool)}
                className="ml-0.5 rounded-sm hover:bg-muted-foreground/20 p-0.5"
                aria-label={`Remove ${tool}`}
              >
                <XIcon className="size-2.5" />
              </button>
            </Badge>
          ))}
          <input
            id="skill-tools"
            className="flex-1 min-w-[80px] bg-transparent text-sm outline-none placeholder:text-muted-foreground/50"
            placeholder="Type + Enter"
            value={toolInput}
            onChange={(e) => setToolInput(e.target.value)}
            onKeyDown={handleToolKeyDown}
          />
        </div>
      </div>

      {/* Switches */}
      <div className="flex flex-col gap-2 pt-1">
        <div className="flex items-center justify-between">
          <Label
            htmlFor="skill-disable-invocation"
            className="cursor-pointer text-xs"
          >
            Disable model invocation
          </Label>
          <Switch
            id="skill-disable-invocation"
            checked={value.disable_model_invocation ?? false}
            onCheckedChange={(checked) =>
              update({ disable_model_invocation: checked || null })
            }
          />
        </div>
        <div className="flex items-center justify-between">
          <Label
            htmlFor="skill-user-invocable"
            className="cursor-pointer text-xs"
          >
            User invocable
          </Label>
          <Switch
            id="skill-user-invocable"
            checked={value.user_invocable ?? false}
            onCheckedChange={(checked) =>
              update({ user_invocable: checked || null })
            }
          />
        </div>
      </div>

      {/* Advanced YAML */}
      <Collapsible open={advancedOpen} onOpenChange={setAdvancedOpen}>
        <CollapsibleTrigger asChild>
          <button
            type="button"
            className={cn(
              "flex w-full items-center gap-1.5 text-xs font-medium text-muted-foreground",
              "hover:text-foreground transition-colors pt-1",
            )}
          >
            {advancedOpen ? (
              <ChevronDownIcon className="size-3.5" />
            ) : (
              <ChevronRightIcon className="size-3.5" />
            )}
            Advanced YAML
          </button>
        </CollapsibleTrigger>
        <CollapsibleContent className="mt-1.5">
          <Textarea
            id="skill-extra-yaml"
            rows={4}
            placeholder={"context: fork\nagent: Explore"}
            value={value.extra_yaml ?? ""}
            onChange={(e) => update({ extra_yaml: e.target.value || null })}
            className="font-mono text-xs resize-none"
          />
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}
