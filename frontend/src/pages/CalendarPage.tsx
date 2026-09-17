import { useMemo, useState } from "react";
import { useCalendar, useCreateEvent, useDeleteEvent } from "../api/events";
import type { CalendarItem } from "../api/types";
import EventForm from "../components/EventForm";

function startOfMonth(year: number, month: number): Date {
  return new Date(year, month, 1, 0, 0, 0, 0);
}

function endOfMonth(year: number, month: number): Date {
  return new Date(year, month + 1, 1, 0, 0, 0, 0);
}

function dayKey(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

const monthNames = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

const weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

export default function CalendarPage() {
  const today = new Date();
  const [year, setYear] = useState(today.getFullYear());
  const [month, setMonth] = useState(today.getMonth());
  const [creating, setCreating] = useState(false);

  const from = startOfMonth(year, month);
  const to = endOfMonth(year, month);
  const calendar = useCalendar(from.toISOString(), to.toISOString());
  const createEvent = useCreateEvent();
  const deleteEvent = useDeleteEvent();

  const itemsByDay = useMemo(() => {
    const map = new Map<string, CalendarItem[]>();
    for (const item of calendar.data ?? []) {
      const key = dayKey(new Date(item.start_time));
      const list = map.get(key) ?? [];
      list.push(item);
      map.set(key, list);
    }
    return map;
  }, [calendar.data]);

  const gridStart = new Date(year, month, 1);
  gridStart.setDate(gridStart.getDate() - gridStart.getDay());

  const cells: Date[] = [];
  for (let i = 0; i < 42; i += 1) {
    const d = new Date(gridStart);
    d.setDate(gridStart.getDate() + i);
    cells.push(d);
  }

  const shift = (delta: number) => {
    const next = new Date(year, month + delta, 1);
    setYear(next.getFullYear());
    setMonth(next.getMonth());
  };

  return (
    <div>
      <div className="page-header">
        <h2>
          {monthNames[month]} {year}
        </h2>
        <div style={{ display: "flex", gap: 6 }}>
          <button onClick={() => shift(-1)}>← Prev</button>
          <button
            onClick={() => {
              setYear(today.getFullYear());
              setMonth(today.getMonth());
            }}
          >
            Today
          </button>
          <button onClick={() => shift(1)}>Next →</button>
          <button className="primary" onClick={() => setCreating((v) => !v)}>
            {creating ? "Close" : "New event"}
          </button>
        </div>
      </div>

      {creating && (
        <EventForm
          onCancel={() => setCreating(false)}
          submitting={createEvent.isPending}
          error={createEvent.error ? (createEvent.error as Error).message : null}
          onSubmit={(body) =>
            createEvent.mutate(body, { onSuccess: () => setCreating(false) })
          }
        />
      )}

      {calendar.error && <div className="error">{(calendar.error as Error).message}</div>}
      {deleteEvent.error && <div className="error">{(deleteEvent.error as Error).message}</div>}

      <div className="calendar-grid" style={{ marginBottom: 4 }}>
        {weekdays.map((day) => (
          <div key={day} className="muted" style={{ fontSize: "0.75rem", textAlign: "center" }}>
            {day}
          </div>
        ))}
      </div>

      <div className="calendar-grid">
        {cells.map((date) => {
          const key = dayKey(date);
          const items = itemsByDay.get(key) ?? [];
          const otherMonth = date.getMonth() !== month;
          const isToday = dayKey(today) === key;
          return (
            <div
              key={key}
              className={`calendar-cell${otherMonth ? " other-month" : ""}`}
              style={isToday ? { borderColor: "var(--primary)" } : undefined}
            >
              <div className="day-number">{date.getDate()}</div>
              {items.map((item) => (
                <div
                  key={`${item.kind}-${item.id}`}
                  className={`calendar-item${item.kind === "task" ? " kind-task" : ""}`}
                  title={`${item.title} (${item.kind})${
                    item.kind === "task" ? " — from task, read-only here" : ""
                  }`}
                >
                  {new Date(item.start_time).toLocaleTimeString([], {
                    hour: "2-digit",
                    minute: "2-digit",
                  })}{" "}
                  {item.title}
                  {item.kind === "event" && (
                    <button
                      style={{
                        float: "right",
                        border: "none",
                        background: "transparent",
                        padding: "0 2px",
                        color: "var(--danger)",
                      }}
                      title="Delete event"
                      onClick={() => deleteEvent.mutate(item.id)}
                    >
                      ×
                    </button>
                  )}
                </div>
              ))}
            </div>
          );
        })}
      </div>

      <p className="muted" style={{ marginTop: 12 }}>
        Tasks with a due date or scheduled time appear automatically (amber). Events are indigo.
        Use "To calendar" on a task to materialize a real, editable event.
      </p>
    </div>
  );
}