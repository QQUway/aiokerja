import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  useDocument,
  useDocumentChunks,
  useReindexDocument,
} from "../api/documents";
import { useExtractDatasheet } from "../api/products";
import { DEVICE_TYPES, type DeviceType } from "../api/types";

function deviceTypeLabel(value: string) {
  return value.replace(/_/g, " ");
}

export default function DocumentDetailPage() {
  const { id } = useParams<{ id: string }>();
  const document = useDocument(id);
  const chunks = useDocumentChunks(id);
  const reindex = useReindexDocument();
  const extractDatasheet = useExtractDatasheet();
  const [deviceType, setDeviceType] = useState<DeviceType | "">("");

  if (document.isLoading) return <div className="empty">Loading…</div>;
  if (document.error) return <div className="error">{(document.error as Error).message}</div>;
  if (!document.data) return null;

  const doc = document.data.document;

  return (
    <div>
      <div className="page-header">
        <h2>{doc.title}</h2>
        <div style={{ display: "flex", gap: 6 }}>
          <button onClick={() => reindex.mutate(doc.id)} disabled={reindex.isPending}>
            {reindex.isPending ? "Re-indexing…" : "Re-index"}
          </button>
          <a href={`/api/documents/${doc.id}/raw`} target="_blank" rel="noreferrer">
            <button>View extracted text</button>
          </a>
          <Link to="/documents">
            <button>Back</button>
          </Link>
        </div>
      </div>

      {reindex.error && <div className="error">{(reindex.error as Error).message}</div>}

      <div className="card">
        <h3>Datasheet extraction</h3>
        <p className="muted">
          Extract structured specs (brand, model, spec attributes) from this AIDC hardware
          datasheet and save it as a comparable product.
        </p>
        <div className="toolbar">
          <select
            value={deviceType}
            onChange={(e) => setDeviceType(e.target.value as DeviceType | "")}
          >
            <option value="">Auto-detect device type</option>
            {DEVICE_TYPES.map((dt) => (
              <option key={dt} value={dt}>
                {deviceTypeLabel(dt)}
              </option>
            ))}
          </select>
          <button
            className="primary"
            disabled={extractDatasheet.isPending || doc.status !== "indexed"}
            onClick={() =>
              extractDatasheet.mutate({
                documentId: doc.id,
                deviceType: deviceType || undefined,
              })
            }
          >
            {extractDatasheet.isPending ? "Extracting…" : "Extract datasheet"}
          </button>
        </div>
        {doc.status !== "indexed" && (
          <div className="muted">Extraction needs the document to finish indexing first.</div>
        )}
        {extractDatasheet.error && (
          <div className="error">{(extractDatasheet.error as Error).message}</div>
        )}
        {extractDatasheet.data && (
          <div className="empty">
            Saved as product “{extractDatasheet.data.name}” (
            {deviceTypeLabel(extractDatasheet.data.device_type ?? "other")}).{" "}
            <Link to="/products">View products</Link>
          </div>
        )}
      </div>

      <div className="card">
        <table>
          <tbody>
            <tr>
              <th>Filename</th>
              <td>{doc.filename}</td>
            </tr>
            <tr>
              <th>Type</th>
              <td>{doc.doc_type}</td>
            </tr>
            <tr>
              <th>Status</th>
              <td>
                <span className={`badge ${doc.status}`}>{doc.status}</span>
                {doc.error && <span className="muted"> — {doc.error}</span>}
              </td>
            </tr>
            <tr>
              <th>Uploaded</th>
              <td>{new Date(doc.upload_date).toLocaleString()}</td>
            </tr>
            <tr>
              <th>Chunks</th>
              <td>{document.data.chunk_count}</td>
            </tr>
            <tr>
              <th>Extracted characters</th>
              <td>{document.data.raw_text_length}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <h3>Chunks</h3>
      {chunks.isLoading && <div className="empty">Loading chunks…</div>}
      {chunks.error && <div className="error">{(chunks.error as Error).message}</div>}
      {chunks.data && chunks.data.length === 0 && (
        <div className="empty">
          No chunks yet. If the status is “failed”, check the error above; otherwise re-index.
        </div>
      )}
      {chunks.data?.map((chunk) => (
        <details className="card" key={chunk.id}>
          <summary>
            Chunk {chunk.chunk_index} · {chunk.token_count} tokens
          </summary>
          <p style={{ whiteSpace: "pre-wrap" }}>{chunk.chunk_text}</p>
        </details>
      ))}
    </div>
  );
}