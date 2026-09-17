import { Link } from "react-router-dom";
import { useDashboard } from "../api/dashboard";
import StatusBadge from "../components/StatusBadge";

export default function Dashboard() {
  const { data, isLoading, error } = useDashboard();

  if (isLoading) return <div className="empty">Loading dashboard…</div>;
  if (error) return <div className="error">{(error as Error).message}</div>;
  if (!data) return null;

  return (
    <div>
      <div className="page-header">
        <h2>Dashboard</h2>
      </div>

      <div className="grid">
        <div className="card">
          <div className="muted">Open tasks</div>
          <div className="stat">
            {data.tasks.total - (data.tasks.by_status.done ?? 0) - (data.tasks.by_status.cancelled ?? 0)}
          </div>
          <div className="muted">of {data.tasks.total} total</div>
        </div>
        <div className="card">
          <div className="muted">Due today</div>
          <div className="stat">{data.tasks.today_count}</div>
        </div>
        <div className="card">
          <div className="muted">Overdue</div>
          <div className="stat" style={{ color: data.tasks.overdue_count ? "#dc2626" : undefined }}>
            {data.tasks.overdue_count}
          </div>
        </div>
        <div className="card">
          <div className="muted">Upcoming</div>
          <div className="stat">{data.tasks.upcoming_count}</div>
        </div>
        <div className="card">
          <div className="muted">Projects</div>
          <div className="stat">{data.projects_count}</div>
        </div>
        <div className="card">
          <div className="muted">Documents indexed</div>
          <div className="stat">
            {data.indexed_documents}
            <span className="muted" style={{ fontSize: "1rem" }}>
              {" "}
              / {data.documents_count}
            </span>
          </div>
        </div>
      </div>

      <div className="card">
        <h3>Next tasks</h3>
        {data.recent_tasks.length === 0 ? (
          <div className="empty">No open tasks. Create one from the Tasks page.</div>
        ) : (
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
              {data.recent_tasks.map((task) => (
                <tr key={task.id}>
                  <td>
                    <Link to="/tasks">{task.title}</Link>
                  </td>
                  <td>
                    <StatusBadge status={task.status} />
                  </td>
                  <td>{task.priority}</td>
                  <td>{task.due_date ? new Date(task.due_date).toLocaleString() : "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="card">
        <h3>Upcoming events (14 days)</h3>
        {data.upcoming_events.length === 0 ? (
          <div className="empty">Nothing scheduled.</div>
        ) : (
          <ul>
            {data.upcoming_events.map((event) => (
              <li key={event.id}>
                <strong>{new Date(event.start_time).toLocaleString()}</strong> — {event.title}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}