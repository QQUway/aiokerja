import { useState } from "react";
import {
  useCreateTask,
  useDeleteTask,
  useMaterializeEvent,
  useTasks,
  useUpdateTask,
} from "../api/tasks";
import { useProjects } from "../api/projects";
import type { TaskDetail, TaskListItem } from "../api/types";
import TaskForm from "../components/TaskForm";
import PriorityBadge from "../components/PriorityBadge";
import StatusBadge from "../components/StatusBadge";

export default function TasksPage() {
  const [status, setStatus] = useState("");
  const [search, setSearch] = useState("");
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState<TaskDetail | null>(null);

  const { data, isLoading, error } = useTasks({
    status: status || undefined,
    search: search || undefined,
    limit: 200,
  });
  const projects = useProjects();
  const createTask = useCreateTask();
  const updateTask = useUpdateTask();
  const deleteTask = useDeleteTask();
  const materialize = useMaterializeEvent();

  const tasks: TaskListItem[] = data?.items ?? [];

  return (
    <div>
      <div className="page-header">
        <h2>Tasks</h2>
        <button className="primary" onClick={() => setCreating((v) => !v)}>
          {creating ? "Close" : "New task"}
        </button>
      </div>

      {creating && (
        <TaskForm
          projects={projects.data ?? []}
          onCancel={() => setCreating(false)}
          submitting={createTask.isPending}
          error={createTask.error ? (createTask.error as Error).message : null}
          onSubmit={(body) =>
            createTask.mutate(body as never, { onSuccess: () => setCreating(false) })
          }
        />
      )}

      {editing && (
        <TaskForm
          initial={editing}
          projects={projects.data ?? []}
          onCancel={() => setEditing(null)}
          submitting={updateTask.isPending}
          error={updateTask.error ? (updateTask.error as Error).message : null}
          onSubmit={(body) =>
            updateTask.mutate(
              { id: editing.id, body: body as never },
              { onSuccess: () => setEditing(null) },
            )
          }
        />
      )}

      <div className="toolbar">
        <select value={status} onChange={(e) => setStatus(e.target.value)}>
          <option value="">All statuses</option>
          <option value="todo">Todo</option>
          <option value="in_progress">In progress</option>
          <option value="done">Done</option>
          <option value="cancelled">Cancelled</option>
        </select>
        <input
          placeholder="Search tasks…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        {data && <span className="muted">{data.total} task(s)</span>}
      </div>

      {isLoading && <div className="empty">Loading…</div>}
      {error && <div className="error">{(error as Error).message}</div>}

      {tasks.length > 0 && (
        <table>
          <thead>
            <tr>
              <th>Title</th>
              <th>Status</th>
              <th>Priority</th>
              <th>Due</th>
              <th>Tags</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {tasks.map((task) => (
              <tr key={task.id}>
                <td>
                  <a
                    href="#"
                    onClick={(e) => {
                      e.preventDefault();
                      setEditing({ ...task, subtasks: [] });
                    }}
                  >
                    {task.title}
                  </a>
                </td>
                <td>
                  <StatusBadge status={task.status} />
                </td>
                <td>
                  <PriorityBadge priority={task.priority} />
                </td>
                <td>{task.due_date ? new Date(task.due_date).toLocaleString() : "—"}</td>
                <td>
                  {task.tags.map((tag) => (
                    <span key={tag.id} className="tag-chip" style={{ backgroundColor: tag.color }}>
                      {tag.name}
                    </span>
                  ))}
                </td>
                <td style={{ whiteSpace: "nowrap" }}>
                  <button
                    onClick={() =>
                      updateTask.mutate({
                        id: task.id,
                        body: { status: task.status === "done" ? "todo" : "done" },
                      })
                    }
                  >
                    {task.status === "done" ? "Reopen" : "Complete"}
                  </button>{" "}
                  <button
                    title="Create a calendar event from this task's scheduled time"
                    disabled={!task.due_date && !task.scheduled_start}
                    onClick={() => materialize.mutate(task.id)}
                  >
                    To calendar
                  </button>{" "}
                  <button
                    className="danger"
                    onClick={() => {
                      if (confirm(`Delete task "${task.title}"?`)) deleteTask.mutate(task.id);
                    }}
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {!isLoading && tasks.length === 0 && (
        <div className="empty">No tasks match this filter.</div>
      )}
    </div>
  );
}