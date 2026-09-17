import { useState } from "react";
import { Link } from "react-router-dom";
import { useSearch } from "../api/search";
import PriorityBadge from "../components/PriorityBadge";
import StatusBadge from "../components/StatusBadge";

export default function SearchPage() {
  const [query, setQuery] = useState("");
  const search = useSearch(query);

  return (
    <div>
      <div className="page-header">
        <h2>Global search</h2>
      </div>
      <div className="toolbar">
        <input
          autoFocus
          placeholder="Search tasks and documents…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          style={{ minWidth: 300 }}
        />
      </div>

      {!query.trim() && <div className="empty">Type at least 2 characters to search.</div>}
      {search.isLoading && <div className="empty">Searching…</div>}
      {search.error && <div className="error">{(search.error as Error).message}</div>}

      {search.data && query.trim() && (
        <>
          <h3>Tasks ({search.data.tasks.length})</h3>
          {search.data.tasks.length === 0 ? (
            <div className="empty">No matching tasks.</div>
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
                {search.data.tasks.map((task) => (
                  <tr key={task.id}>
                    <td>
                      <Link to="/tasks">{task.title}</Link>
                    </td>
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

          <h3 style={{ marginTop: 16 }}>Documents ({search.data.documents.length})</h3>
          {search.data.documents.length === 0 ? (
            <div className="empty">No matching documents.</div>
          ) : (
            <table>
              <thead>
                <tr>
                  <th>Title</th>
                  <th>Filename</th>
                  <th>Status</th>
                  <th>Uploaded</th>
                </tr>
              </thead>
              <tbody>
                {search.data.documents.map((doc) => (
                  <tr key={doc.id}>
                    <td>
                      <Link to={`/documents/${doc.id}`}>{doc.title}</Link>
                    </td>
                    <td>{doc.filename}</td>
                    <td>
                      <span className={`badge ${doc.status}`}>{doc.status}</span>
                    </td>
                    <td>{new Date(doc.upload_date).toLocaleString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </>
      )}
    </div>
  );
}