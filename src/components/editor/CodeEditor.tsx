import * as React from "react";
import CodeMirrorEditor from "@uiw/react-codemirror";
import { javascript } from "@codemirror/lang-javascript";
import { python } from "@codemirror/lang-python";
import { json } from "@codemirror/lang-json";
import { yaml } from "@codemirror/lang-yaml";
import { markdown } from "@codemirror/lang-markdown";
import { rust } from "@codemirror/lang-rust";
import { html } from "@codemirror/lang-html";
import { css } from "@codemirror/lang-css";
import type { Extension } from "@codemirror/state";

function getLanguageExtension(filename: string): Extension[] {
  const ext = filename.split(".").pop()?.toLowerCase() ?? "";
  switch (ext) {
    case "js":
    case "jsx":
      return [javascript({ jsx: true })];
    case "ts":
    case "tsx":
      return [javascript({ jsx: true, typescript: true })];
    case "py":
      return [python()];
    case "json":
      return [json()];
    case "yaml":
    case "yml":
      return [yaml()];
    case "md":
      return [markdown()];
    case "rs":
      return [rust()];
    case "html":
      return [html()];
    case "css":
      return [css()];
    default:
      return [];
  }
}

interface CodeEditorProps {
  value: string;
  onChange?: (value: string) => void;
  filename: string;
  readOnly?: boolean;
}

export function CodeEditor({ value, onChange, filename, readOnly }: CodeEditorProps) {
  const extensions = React.useMemo(() => getLanguageExtension(filename), [filename]);

  return (
    <CodeMirrorEditor
      value={value}
      onChange={onChange}
      extensions={extensions}
      readOnly={readOnly}
      theme="light"
      className="h-full min-h-0 flex-1 overflow-auto"
      basicSetup={{
        lineNumbers: true,
        foldGutter: true,
        highlightActiveLine: true,
        tabSize: 2,
      }}
    />
  );
}
