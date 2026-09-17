import { useParams } from "react-router-dom";
import { useProject } from "../api/projects";
import { useTasks } from "../api/tasks";
import PriorityBadge from "../components/PriorityBadge";
import StatusBadge from "../components/StatusBadge";

export default function ProjectDetailPage() {
  const { id } = useParams<{ id: string }>();
  const project = useProject(id);
  const tasks = useTasks({ project_id: id, limit: 500 });

  if (project.isLoading) return <div className="empty">Loading…</div>;
  if (project.error || !project.data) {
    return <div className="error">{(project.error as Error)?.message ?? "Project not found"}</div>;
  }

  return (
    <div>
      <div className="page-header">
        <h2 style={{ color: project.data.project.color }}>{project.data.project.name}</h2>
        <span className="muted">
          {project.data.task_count} task(s), {tasks.data?.total ?? 0} shown
        </span>
      </div>
      <p className="muted">{project.data.project.description || "No description"}</p>

      {tasks.error && <div className="error">{(tasks.error as Error).message}</div>}
      {tasks.data && tasks.data.items.length === 0 && <div className="empty">No tasks in this project.</div>}

      {tasks.data && tasks.data.items.length > 0 && (
        <table>
          <thead>
            <tr>
              <th>Title</th>
              <th>Status</th>
              <th>Priority</th>
              <th>Due</th>
            </tr>
          </thead>
          <tbody>
            {tasks.data.items.map((task) => (
              <tr key={task.id}>
                <td>{task.title}</td>
                <td>
                  <StatusBadge status={task.status} />
                </td>
                <td>
                  <PriorityBadge priority={task.priority} />
                </td>
                <td>{task.due_date ? new Date(task.due_date).toLocaleString() : "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}