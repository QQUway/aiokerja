import { useState } from "react";
import type { TaskDetail, TaskStatus, Priority } from "../api/types";
import type { NewTask, UpdateTask } from "../api/tasks";

interface Props {
  initial?: TaskDetail;
  projects?: { id: string; name: string }[];
  onCancel: () => void;
  onSubmit: (body: NewTask | UpdateTask) => void;
  submitting?: boolean;
  error?: string | null;
}

function toLocal(value: string | null | undefined): string {
  if (!value) return "";
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

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
  const [status, setStatus] = useState(initial?.status ?? "todo");
  const [priority, setPriority] = useState(initial?.priority ?? "medium");
  const [dueDate, setDueDate] = useState(toLocal(initial?.due_date));
  const [scheduledStart, setScheduledStart] = useState(toLocal(initial?.scheduled_start));
  const [scheduledEnd, setScheduledEnd] = useState(toLocal(initial?.scheduled_end));
  const [projectId, setProjectId] = useState(initial?.project_id ?? "");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const base = {
      title,
      description: description || null,
      status: status as NewTask["status"],
      priority: priority as NewTask["priority"],
      due_date: dueDate ? new Date(dueDate).toISOString() : null,
      scheduled_start: scheduledStart ? new Date(scheduledStart).toISOString() : null,
      scheduled_end: scheduledEnd ? new Date(scheduledEnd).toISOString() : null,
      project_id: projectId || null,
    };
    onSubmit(initial ? { ...base } : { ...base, tag_ids: [] });
  };

  return (
    <form className="card" onSubmit={handleSubmit}>
      <div className="form-row">
        <label>Title *</label>
        <input value={title} onChange={(e) => setTitle(e.target.value)} required autoFocus />
      </div>
      <div className="form-row">
        <label>Description</label>
        <textarea value={description} onChange={(e) => setDescription(e.target.value)} />
      </div>
      <div className="form-row">
        <label>Status</label>
        <select value={status} onChange={(e) => setStatus(e.target.value as TaskStatus)}>
          <option value="todo">Todo</option>
          <option value="in_progress">In progress</option>
          <option value="done">Done</option>
          <option value="cancelled">Cancelled</option>
        </select>
      </div>
      <div className="form-row">
        <label>Priority</label>
        <select value={priority} onChange={(e) => setPriority(e.target.value as Priority)}>
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
          <option value="urgent">Urgent</option>
        </select>
      </div>
      <div className="form-row">
        <label>Due date</label>
        <input type="datetime-local" value={dueDate} onChange={(e) => setDueDate(e.target.value)} />
      </div>
      <div className="form-row">
        <label>Scheduled start</label>
        <input
          type="datetime-local"
          value={scheduledStart}
          onChange={(e) => setScheduledStart(e.target.value)}
        />
      </div>
      <div className="form-row">
        <label>Scheduled end</label>
        <input
          type="datetime-local"
          value={scheduledEnd}
          onChange={(e) => setScheduledEnd(e.target.value)}
        />
      </div>
      <div className="form-row">
        <label>Project</label>
        <select value={projectId} onChange={(e) => setProjectId(e.target.value)}>
          <option value="">— none —</option>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </div>
      {error && <div className="error">{error}</div>}
      <div className="form-actions">
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