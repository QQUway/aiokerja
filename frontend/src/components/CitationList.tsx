import type { Citation } from "../api/types";

export default function CitationList({ citations }: { citations: Citation[] }) {
  if (!citations || citations.length === 0) return null;
  return (
    <details className="citations">
      <summary>
        {citations.length} source{citations.length === 1 ? "" : "s"}
      </summary>
      {citations.map((citation, index) => (
        <div className="citation" key={`${citation.document_id}-${index}`}>
          <div>
            <strong>[ref {index}]</strong> {citation.title}{" "}
            <span className="muted">(score {citation.score.toFixed(3)})</span>
          </div>
          <div>{citation.chunk_text}</div>
        </div>
      ))}
    </details>
  );
}