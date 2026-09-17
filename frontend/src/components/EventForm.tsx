import { useState } from "react";
import type { CalendarEvent } from "../api/types";
import type { NewEvent } from "../api/events";

interface Props {
  initial?: CalendarEvent;
  onCancel: () => void;
  onSubmit: (body: NewEvent) => void;
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

export default function EventForm({ initial, onCancel, onSubmit, submitting, error }: Props) {
  const [title, setTitle] = useState(initial?.title ?? "");
  const [description, setDescription] = useState(initial?.description ?? "");
  const [start, setStart] = useState(toLocal(initial?.start_time));
  const [end, setEnd] = useState(toLocal(initial?.end_time));
  const [allDay, setAllDay] = useState(initial?.all_day ?? false);
  const [reminder, setReminder] = useState(
    initial?.reminder_minutes != null ? String(initial.reminder_minutes) : "",
  );

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit({
      title,
      description: description || null,
      start_time: start ? new Date(start).toISOString() : undefined,
      end_time: end ? new Date(end).toISOString() : undefined,
      all_day: allDay,
      reminder_minutes: reminder ? Number(reminder) : null,
    });
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
        <label>Start *</label>
        <input
          type="datetime-local"
          value={start}
          onChange={(e) => setStart(e.target.value)}
          required
        />
      </div>
      <div className="form-row">
        <label>End *</label>
        <input type="datetime-local" value={end} onChange={(e) => setEnd(e.target.value)} required />
      </div>
      <div className="form-row">
        <label>
          <input
            type="checkbox"
            checked={allDay}
            onChange={(e) => setAllDay(e.target.checked)}
            style={{ width: "auto", marginRight: 6 }}
          />
          All day
        </label>
      </div>
      <div className="form-row">
        <label>Reminder (minutes before)</label>
        <input
          type="number"
          min={0}
          value={reminder}
          onChange={(e) => setReminder(e.target.value)}
        />
      </div>
      {error && <div className="error">{error}</div>}
      <div className="form-actions">
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
        <button type="submit" className="primary" disabled={submitting || !title.trim()}>
          {submitting ? "Saving…" : initial ? "Save" : "Create event"}
        </button>
      </div>
    </form>
  );
}