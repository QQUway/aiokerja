import { useMemo, useState } from "react";
import type { TaskDetail, TaskStatus, Priority } from "../api/types";
import type { NewTask, UpdateTask } from "../api/tasks";
import { formatDuePreview, matchProject, parseNaturalDate } from "../utils/taskInput";

interface Props {
  initial?: TaskDetail;
  projects?: { id: string; name: string }[];
  onCancel: () => void;
  onSubmit: (body: NewTask | UpdateTask) => void;
  submitting?: boolean;
  error?: string | null;
}

/** Render an existing ISO timestamp as editable "YYYY-MM-DD HH:MM" text. */
function toEditable(value: string | null | undefined): string {
  if (!value) return "";
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

const PRIORITIES: Priority[] = ["low", "medium", "high", "urgent"];

export default function TaskForm({
  initial,
  projects = [],
  onCancel,
  onSubmit,
  submitting,
  error,
}: Props) {
  const [title, setTitle] = useState(initial?.title ?? "");
  const [description, setDescription] = useState(initial?.description ?? "");
  const [status, setStatus] = useState<TaskStatus>(initial?.status ?? "todo");
  const [priority, setPriority] = useState<Priority>(initial?.priority ?? "medium");
  const [dueText, setDueText] = useState(toEditable(initial?.due_date));
  const [scheduledStart, setScheduledStart] = useState(toEditable(initial?.scheduled_start));
  const [scheduledEnd, setScheduledEnd] = useState(toEditable(initial?.scheduled_end));
  const [projectText, setProjectText] = useState(
    () => projects.find((p) => p.id === initial?.project_id)?.name ?? "",
  );
  const [dateError, setDateError] = useState<string | null>(null);

  const duePreview = useMemo(
    () => (dueText.trim() ? parseNaturalDate(dueText.trim()) : null),
    [dueText],
  );
  const projectMatch = useMemo(
    () => (projectText.trim() ? matchProject(projectText.trim(), projects) : null),
    [projectText, projects],
  );

  const doSubmit = () => {
    if (!title.trim() || submitting) return;
    if (dueText.trim() && !duePreview) {
      setDateError(`Couldn't understand "${dueText.trim()}". Try "tomorrow 17:00", "fri", "2026-10-01 14:00".`);
      return;
    }
    setDateError(null);
    const iso = (t: string) => {
      const t2 = t.trim();
      if (!t2) return null;
      // Accept raw "YYYY-MM-DDTHH:MM" (datetime-local shape) as well as natural text.
      const d = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/.test(t2) ? new Date(t2) : parseNaturalDate(t2);
      return d && !Number.isNaN(d.getTime()) ? d.toISOString() : null;
    };
    const base = {
      title: title.trim(),
      description: description.trim() || null,
      status,
      priority,
      due_date: iso(dueText),
      scheduled_start: iso(scheduledStart),
      scheduled_end: iso(scheduledEnd),
      project_id: projectText.trim() ? (projectMatch?.id ?? null) : null,
    };
    onSubmit(initial ? { ...base } : { ...base, tag_ids: [] });
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    doSubmit();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    // Ctrl/Cmd+Enter saves from anywhere (esp. the description textarea,
    // where plain Enter must insert a newline).
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      doSubmit();
      return;
    }
    // Alt+1..4 jumps priority without touching the mouse.
    if (e.altKey && ["1", "2", "3", "4"].includes(e.key)) {
      e.preventDefault();
      setPriority(PRIORITIES[parseInt(e.key, 10) - 1]);
      return;
    }
    // Esc cancels — but leave <select> dropdowns alone so Esc first
    // closes the dropdown (native behavior) instead of nuking the form.
    if (e.key === "Escape" && !(e.target instanceof HTMLSelectElement)) {
      e.preventDefault();
      onCancel();
    }
  };

  return (
    <form className="card" onSubmit={handleSubmit} onKeyDown={handleKeyDown}>
      <div className="form-row">
        <label htmlFor="task-title">Title *</label>
        <input
          id="task-title"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          required
          autoFocus
          autoComplete="off"
          placeholder="What needs doing?"
        />
      </div>
      <div className="form-row">
        <label htmlFor="task-due">Due <span className="muted">(type it — no calendar clicking needed)</span></label>
        <input
          id="task-due"
          value={dueText}
          onChange={(e) => setDueText(e.target.value)}
          autoComplete="off"
          placeholder="tomorrow 17:00 · fri · 2026-10-01 14:00"
          aria-describedby="task-due-hint"
        />
        <div id="task-due-hint" className="muted" style={{ fontSize: "0.72rem" }} aria-live="polite">
          {dueText.trim() ? (
            duePreview ? (
              <>→ {duePreview.toLocaleString()} ({formatDuePreview(duePreview)})</>
            ) : (
              <span style={{ color: "var(--danger)" }}>unrecognized date — leave blank for none</span>
            )
          ) : (
            <>today · tomorrow · mon–sun · in 3 days · sep 30 · 5pm · blank = none</>
          )}
        </div>
      </div>
      <div className="form-row">
        <span id="task-priority-label" className="muted" style={{ fontSize: "0.78rem" }}>
          Priority <span className="muted">(Alt+1…4)</span>
        </span>
        <div role="radiogroup" aria-labelledby="task-priority-label" style={{ display: "flex", gap: "0.35rem" }}>
          {PRIORITIES.map((p, i) => (
            <label key={p} style={{ display: "flex", alignItems: "center", gap: "0.25rem", cursor: "pointer" }} title={`Alt+${i + 1}`}>
              <input
                type="radio"
                name="priority"
                value={p}
                checked={priority === p}
                onChange={() => setPriority(p)}
              />
              {p}
            </label>
          ))}
        </div>
      </div>
      <div className="form-row">
        <label htmlFor="task-project">Project <span className="muted">(type to filter)</span></label>
        <input
          id="task-project"
          value={projectText}
          onChange={(e) => setProjectText(e.target.value)}
          list="task-project-options"
          autoComplete="off"
          placeholder="— none —"
        />
        <datalist id="task-project-options">
          {projects.map((p) => (
            <option key={p.id} value={p.name} />
          ))}
        </datalist>
        {projectText.trim() && !projectMatch && (
          <div className="muted" style={{ fontSize: "0.72rem" }}>
            no project matches — will be saved without a project
          </div>
        )}
      </div>
      <div className="form-row">
        <label htmlFor="task-desc">Description <span className="muted">(Ctrl+↵ to save)</span></label>
        <textarea
          id="task-desc"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          rows={2}
        />
      </div>
      <details style={{ marginBottom: "0.75rem" }} open={!!initial}>
        <summary style={{ cursor: "pointer", fontSize: "0.78rem" }} className="muted">
          More fields (status, schedule)
        </summary>
        <div style={{ marginTop: "0.6rem" }}>
          <div className="form-row">
            <label htmlFor="task-status">Status</label>
            <select
              id="task-status"
              value={status}
              onChange={(e) => setStatus(e.target.value as TaskStatus)}
            >
              <option value="todo">Todo</option>
              <option value="in_progress">In progress</option>
              <option value="done">Done</option>
              <option value="cancelled">Cancelled</option>
            </select>
          </div>
          <div className="form-row">
            <label htmlFor="task-start">Scheduled start</label>
            <input
              id="task-start"
              value={scheduledStart}
              onChange={(e) => setScheduledStart(e.target.value)}
              autoComplete="off"
              placeholder="same formats as due"
            />
          </div>
          <div className="form-row">
            <label htmlFor="task-end">Scheduled end</label>
            <input
              id="task-end"
              value={scheduledEnd}
              onChange={(e) => setScheduledEnd(e.target.value)}
              autoComplete="off"
              placeholder="same formats as due"
            />
          </div>
        </div>
      </details>
      {(dateError || error) && <div className="error">{dateError ?? error}</div>}
      <div className="form-actions" style={{ alignItems: "center" }}>
        <span className="muted" style={{ marginRight: "auto", fontSize: "0.72rem" }}>
          Ctrl+↵ save · Esc cancel
        </span>
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
        <button type="submit" className="primary" disabled={submitting || !title.trim()}>
          {submitting ? "Saving…" : initial ? "Save" : "Create task"}
        </button>
      </div>
    </form>
  );
}
