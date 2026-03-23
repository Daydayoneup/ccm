import {
  List,
  ActionPanel,
  Action,
  getPreferenceValues,
  showToast,
  Toast,
  Icon,
} from "@raycast/api";
import { useState, useEffect } from "react";

interface Preferences {
  apiToken: string;
  apiPort?: string;
}

interface Project {
  id: string;
  name: string;
  path: string;
  language: string | null;
  pinned: boolean;
  launch_count: number;
}

interface ApiResponse<T> {
  ok: boolean;
  data?: T;
  error?: string;
}

function getBaseUrl(): string {
  const { apiPort } = getPreferenceValues<Preferences>();
  const port = apiPort || "23890";
  return `http://127.0.0.1:${port}`;
}

function getHeaders(): Record<string, string> {
  const { apiToken } = getPreferenceValues<Preferences>();
  return {
    Authorization: `Bearer ${apiToken}`,
    "Content-Type": "application/json",
  };
}

async function fetchProjects(query?: string): Promise<Project[]> {
  const url = new URL(`${getBaseUrl()}/api/projects`);
  if (query) url.searchParams.set("q", query);

  const response = await fetch(url.toString(), { headers: getHeaders() });
  const body: ApiResponse<Project[]> = await response.json();

  if (!body.ok) {
    throw new Error(body.error || `HTTP ${response.status}`);
  }
  return body.data || [];
}

async function launchProject(id: string): Promise<void> {
  const response = await fetch(`${getBaseUrl()}/api/projects/${id}/launch`, {
    method: "POST",
    headers: getHeaders(),
  });
  const body: ApiResponse<null> = await response.json();
  if (!body.ok) {
    throw new Error(body.error || `HTTP ${response.status}`);
  }
}

export default function SearchProjects() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [searchText, setSearchText] = useState("");

  useEffect(() => {
    const timer = setTimeout(() => {
      setIsLoading(true);
      fetchProjects(searchText || undefined)
        .then(setProjects)
        .catch((err) => {
          showToast(Toast.Style.Failure, "Failed to fetch projects", err.message);
          setProjects([]);
        })
        .finally(() => setIsLoading(false));
    }, 200);

    return () => clearTimeout(timer);
  }, [searchText]);

  return (
    <List
      isLoading={isLoading}
      searchBarPlaceholder="Search projects..."
      onSearchTextChange={setSearchText}
      throttle
    >
      {projects.map((project) => (
        <List.Item
          key={project.id}
          icon={project.pinned ? Icon.Pin : Icon.Folder}
          title={project.name}
          subtitle={project.path}
          accessories={[
            ...(project.language ? [{ tag: project.language }] : []),
            { text: `${project.launch_count} launches` },
          ]}
          actions={
            <ActionPanel>
              <Action
                title="Launch Claude Code"
                icon={Icon.Terminal}
                onAction={async () => {
                  try {
                    await launchProject(project.id);
                    showToast(Toast.Style.Success, "Launched", project.name);
                  } catch (err: unknown) {
                    const message = err instanceof Error ? err.message : String(err);
                    showToast(Toast.Style.Failure, "Launch failed", message);
                  }
                }}
              />
              <Action.Open
                title="Open in Finder"
                target={project.path}
                icon={Icon.Finder}
                shortcut={{ modifiers: ["cmd"], key: "o" }}
              />
              <Action.CopyToClipboard
                title="Copy Path"
                content={project.path}
                shortcut={{ modifiers: ["cmd"], key: "c" }}
              />
            </ActionPanel>
          }
        />
      ))}
    </List>
  );
}
