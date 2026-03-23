import type { Root } from "mdast";
import matter from "gray-matter";

/**
 * Remark plugin: converts YAML frontmatter AST nodes into code blocks
 * with lang="skill-frontmatter" so a custom component can render them.
 */
export function remarkFrontmatterCard() {
  return (tree: Root) => {
    for (let i = 0; i < tree.children.length; i++) {
      const node = tree.children[i];
      if (node.type === "yaml") {
        tree.children[i] = {
          type: "code",
          lang: "skill-frontmatter",
          value: node.value,
        };
      }
    }
  };
}

/** Known boolean fields in skill frontmatter */
const BOOL_FIELDS = ["disable-model-invocation", "user-invocable"];

/** Parse YAML string into key-value pairs */
function parseYaml(yamlStr: string): Record<string, unknown> {
  try {
    const { data } = matter(`---\n${yamlStr}\n---\n`);
    return data;
  } catch {
    return {};
  }
}

/** Frontmatter card component rendered in preview mode */
export function FrontmatterCard({ yaml: yamlStr }: { yaml: string }) {
  const data = parseYaml(yamlStr);

  if (Object.keys(data).length === 0) {
    return (
      <div style={styles.card}>
        <div style={styles.header}>⚙ Skill Metadata</div>
        <div style={styles.empty}>No valid frontmatter</div>
      </div>
    );
  }

  const entries = Object.entries(data);

  return (
    <div style={styles.card}>
      <div style={styles.header}>⚙ Skill Metadata</div>
      <div style={styles.body}>
        {entries.map(([key, value]) => {
          if (BOOL_FIELDS.includes(key)) {
            const checked = Boolean(value);
            return (
              <div key={key} style={styles.boolRow}>
                <span style={{ marginRight: 6 }}>{checked ? "☑" : "☐"}</span>
                <span style={styles.boolLabel}>{key}</span>
              </div>
            );
          }

          return (
            <div key={key} style={styles.row}>
              <span style={styles.label}>{key}</span>
              <span style={styles.value}>
                {Array.isArray(value)
                  ? value.map((item, i) => (
                      <span key={i} style={styles.badge}>
                        {String(item)}
                      </span>
                    ))
                  : String(value ?? "")}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

/**
 * Custom code component for MDEditor preview.
 * Intercepts code blocks with lang="skill-frontmatter" and renders FrontmatterCard.
 * Pass this to MDEditor's previewOptions.components.code
 */
export function skillPreviewCode({
  className,
  children,
  ...props
}: React.ComponentPropsWithoutRef<"code"> & { children?: React.ReactNode }) {
  if (className === "language-skill-frontmatter") {
    const yamlStr = String(children).replace(/\n$/, "");
    return <FrontmatterCard yaml={yamlStr} />;
  }
  return (
    <code className={className} {...props}>
      {children}
    </code>
  );
}

/** Inline styles (used because this renders inside MDEditor's preview which may not have Tailwind) */
const styles: Record<string, React.CSSProperties> = {
  card: {
    border: "1px solid #e2e8f0",
    borderRadius: 8,
    overflow: "hidden",
    marginBottom: 16,
    fontFamily: "system-ui, -apple-system, sans-serif",
    fontSize: 13,
  },
  header: {
    background: "#f8fafc",
    borderBottom: "1px solid #e2e8f0",
    padding: "8px 14px",
    fontWeight: 600,
    fontSize: 12,
    color: "#64748b",
    textTransform: "uppercase" as const,
    letterSpacing: "0.05em",
  },
  body: {
    padding: "10px 14px",
  },
  row: {
    display: "flex",
    alignItems: "baseline",
    padding: "4px 0",
    gap: 12,
  },
  label: {
    flexShrink: 0,
    width: 140,
    color: "#94a3b8",
    fontSize: 12,
    fontWeight: 500,
  },
  value: {
    color: "#1e293b",
    fontSize: 13,
    display: "flex",
    flexWrap: "wrap" as const,
    gap: 4,
  },
  badge: {
    display: "inline-block",
    background: "#eff6ff",
    color: "#3b82f6",
    borderRadius: 4,
    padding: "1px 8px",
    fontSize: 12,
    fontWeight: 500,
    border: "1px solid #bfdbfe",
  },
  boolRow: {
    display: "flex",
    alignItems: "center",
    padding: "3px 0",
  },
  boolLabel: {
    fontSize: 13,
    color: "#475569",
  },
  empty: {
    padding: "12px 14px",
    color: "#94a3b8",
    fontStyle: "italic",
  },
};
