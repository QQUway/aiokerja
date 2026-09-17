import { useState } from "react";
import { Link } from "react-router-dom";
import { useCreateProject, useDeleteProject, useProjects } from "../api/projects";

export default function ProjectsPage() {
  const { data, isLoading, error } = useProjects();
  const createProject = useCreateProject();
  const deleteProject = useDeleteProject();
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [color, setColor] = useState("#4f46e5");

  return (
    <div>
      <div className="page-header">
        <h2>Projects</h2>
        <button className="primary" onClick={() => setCreating((v) => !v)}>
          {creating ? "Close" : "New project"}
        </button>
      </div>

      {creating && (
        <form
          className="card"
          onSubmit={(e) => {
            e.preventDefault();
            createProject.mutate({ name, color }, { onSuccess: () => {
              setName("");
              setCreating(false);
            }});
          }}
        >
          <div className="form-row">
            <label>Name *</label>
            <input value={name} onChange={(e) => setName(e.target.value)} required autoFocus />
          </div>
          <div className="form-row">
            <label>Color</label>
            <input
              type="color"
              value={color}
              onChange={(e) => setColor(e.target.value)}
              style={{ width: 60, padding: 2 }}
            />
          </div>
          <div className="form-actions">
            <button type="button" onClick={() => setCreating(false)}>
              Cancel
            </button>
            <button type="submit" className="primary" disabled={createProject.isPending || !name.trim()}>
              Create
            </button>
          </div>
        </form>
      )}

      {createProject.error && <div className="error">{(createProject.error as Error).message}</div>}
      {deleteProject.error && <div className="error">{(deleteProject.error as Error).message}</div>}
      {isLoading && <div className="empty">Loading…</div>}
      {error && <div className="error">{(error as Error).message}</div>}

      {data && data.length === 0 && !isLoading && (
        <div className="empty">No projects yet. Create one above.</div>
      )}

      <div className="grid">
        {data?.map((project) => (
          <div className="card" key={project.id} style={{ borderTop: `3px solid ${project.color}` }}>
            <h3>
              <Link to={`/projects/${project.id}`}>{project.name}</Link>
            </h3>
            <div className="muted">{project.description || "No description"}</div>
            <div style={{ marginTop: 8 }}>
              <button
                className="danger"
                onClick={() => {
                  if (confirm(`Delete project "${project.name}"? Tasks inside need manual cleanup.`)) {
                    deleteProject.mutate({ id: project.id, cascade: true });
                  }
                }}
              >
                Delete
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}