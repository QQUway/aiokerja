import { forwardRef, useImperativeHandle, useMemo, useRef, useState } from "react";
import type { NewTask } from "../api/tasks";
import { formatDuePreview, parseQuickAdd } from "../utils/taskInput";

export interface QuickAddHandle {
  focus: () => void;
}

interface Props {
  projects?: { id: string; name: string }[];
  submitting?: boolean;
  error?: string | null;
  onSubmit: (body: NewTask) => void;
}

const HINT =
  "Enter ↵ to add · Esc to clear · try: Buy milk tomorrow !high #project · p1–p4 = priority · due:friday";

const QuickAddTask = forwardRef<QuickAddHandle, Props>(function QuickAddTask(
  { projects = [], submitting, error, onSubmit },
  ref,
) {
  const [value, setValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useImperativeHandle(ref, () => ({ focus: () => inputRef.current?.focus() }), []);

  const parsed = useMemo(() => parseQuickAdd(value, projects), [value, projects]);
  const canSubmit = parsed.title.trim().length > 0 && !submitting;

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && canSubmit) {
      e.preventDefault();
      onSubmit({
        title: parsed.title.trim(),
        priority: parsed.priority ?? "medium",
        due_date: parsed.dueDate ? parsed.dueDate.toISOString() : null,
        project_id: parsed.projectId,
      });
      setValue("");
      // Stay focused for rapid-fire entry — the whole point of quick-add.
      requestAnimationFrame(() => inputRef.current?.focus());
    } else if (e.key === "Escape") {
      setValue("");
      inputRef.current?.blur();
    }
  };

  return (
    <div className="card" style={{ padding: "0.6rem 0.7rem" }}>
      <input
        ref={inputRef}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Add a task… e.g. Write report tomorrow 9am !high"
        aria-label="Quick add task"
        title={HINT}
        autoComplete="off"
        style={{ fontSize: "0.9rem" }}
      />
      {value.trim() && (
        <div
          className="muted"
          style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap", marginTop: "0.4rem" }}
          aria-live="polite"
        >
          {parsed.title.trim() ? (
            <span className="badge" title="Parsed title">
              “{parsed.title.trim()}”
            </span>
          ) : (
            <span className="badge" title="No title left after parsing">
              (no title)
            </span>
          )}
          {parsed.dueDate && (
            <span className="badge in_progress" title={parsed.dueDate.toLocaleString()}>
              📅 {formatDuePreview(parsed.dueDate)}
            </span>
          )}
          {parsed.priority && (
            <span className={`badge ${parsed.priority}`} title="Parsed priority">
              {parsed.priority}
            </span>
          )}
          {parsed.projectName && (
            <span className="badge" title="Parsed project">
              #{parsed.projectName}
            </span>
          )}
          <span style={{ marginLeft: "auto", fontSize: "0.72rem" }}>
            {canSubmit ? "↵ add" : submitting ? "saving…" : "type a title"}
          </span>
        </div>
      )}
      {!value.trim() && (
        <div className="muted" style={{ fontSize: "0.72rem", marginTop: "0.3rem" }}>
          {HINT} · press <kbd>c</kbd> anywhere to jump here
        </div>
      )}
      {error && (
        <div className="error" style={{ marginTop: "0.4rem", marginBottom: 0 }}>
          {error}
        </div>
      )}
    </div>
  );
});

export default QuickAddTask;
