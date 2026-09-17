import { useRef, useState } from "react";
import { Link } from "react-router-dom";
import { useDeleteDocument, useDocuments, useUploadDocument } from "../api/documents";
import type { DocStatus } from "../api/types";

export default function DocumentsPage() {
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState("");
  const fileInput = useRef<HTMLInputElement>(null);
  const documents = useDocuments({ search: search || undefined, status: status || undefined });
  const upload = useUploadDocument();
  const remove = useDeleteDocument();

  return (
    <div>
      <div className="page-header">
        <h2>Documents</h2>
        <div>
          <input
            ref={fileInput}
            type="file"
            accept=".pdf,.txt,.md,.docx"
            multiple
            style={{ display: "none" }}
            onChange={async (e) => {
              const files = Array.from(e.target.files ?? []);
              e.target.value = "";
              for (const file of files) {
                await upload.mutateAsync(file).catch(() => {});
              }
            }}
          />
          <button
            className="primary"
            onClick={() => fileInput.current?.click()}
            disabled={upload.isPending}
          >
            {upload.isPending ? "Uploading…" : "Upload documents"}
          </button>
        </div>
      </div>

      <p className="muted">
        Supported: PDF, TXT, Markdown, DOCX. Text is extracted, chunked, embedded and stored in
        pgvector. The document becomes searchable once status is “indexed”.
      </p>

      {upload.error && <div className="error">{(upload.error as Error).message}</div>}
      {remove.error && <div className="error">{(remove.error as Error).message}</div>}

      <div className="toolbar">
        <input
          placeholder="Search by title or filename…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <select value={status} onChange={(e) => setStatus(e.target.value)}>
          <option value="">All statuses</option>
          <option value="pending">Pending</option>
          <option value="indexing">Indexing</option>
          <option value="indexed">Indexed</option>
          <option value="failed">Failed</option>
        </select>
        <button onClick={() => documents.refetch()}>Refresh</button>
      </div>

      {documents.isLoading && <div className="empty">Loading…</div>}
      {documents.error && <div className="error">{(documents.error as Error).message}</div>}

      {documents.data && documents.data.length === 0 && (
        <div className="empty">No documents yet. Upload one above.</div>
      )}

      {documents.data && documents.data.length > 0 && (
        <table>
          <thead>
            <tr>
              <th>Title</th>
              <th>Type</th>
              <th>Status</th>
              <th>Uploaded</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {documents.data.map((doc) => (
              <tr key={doc.id}>
                <td>
                  <Link to={`/documents/${doc.id}`}>{doc.title}</Link>
                  {doc.error && <div className="error" style={{ marginTop: 4 }}>{doc.error}</div>}
                </td>
                <td>{doc.doc_type}</td>
                <td>
                  <span className={`badge ${doc.status as DocStatus}`}>{doc.status}</span>
                </td>
                <td>{new Date(doc.upload_date).toLocaleString()}</td>
                <td>
                  <button
                    className="danger"
                    onClick={() => {
                      if (confirm(`Delete "${doc.title}" and its chunks?`)) remove.mutate(doc.id);
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
    </div>
  );
}