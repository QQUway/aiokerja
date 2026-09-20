import { useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { useCompareProducts } from "../api/products";
import CitationList from "../components/CitationList";

export default function ProductComparePage() {
  const [params] = useSearchParams();
  const productIds = (params.get("ids") ?? "").split(",").filter(Boolean);
  const compare = useCompareProducts();
  const [question, setQuestion] = useState("");

  useEffect(() => {
    if (productIds.length >= 2) {
      compare.mutate({ productIds });
    }
    // Run once for the ids in the URL; re-runs are user-triggered via the form below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [params.get("ids")]);

  return (
    <div>
      <div className="page-header">
        <h2>Compare products</h2>
        <Link to="/products">
          <button>Back to products</button>
        </Link>
      </div>

      {productIds.length < 2 && (
        <div className="empty">Pick at least 2 products on the Products page to compare.</div>
      )}

      {compare.isPending && <div className="empty">Comparing…</div>}
      {compare.error && <div className="error">{(compare.error as Error).message}</div>}

      {compare.data && (
        <>
          <div className="card">
            <div className="toolbar">
              <input
                placeholder="Ask a specific comparison question…"
                value={question}
                onChange={(e) => setQuestion(e.target.value)}
                style={{ flex: 1 }}
              />
              <button
                className="primary"
                disabled={compare.isPending}
                onClick={() => compare.mutate({ productIds, question: question || undefined })}
              >
                Ask
              </button>
            </div>
            {compare.data.summary && (
              <div className="markdown" style={{ whiteSpace: "pre-wrap" }}>
                {compare.data.summary}
              </div>
            )}
            {!compare.data.summary && (
              <div className="muted">
                No grounded summary available (source documents may not be indexed yet). The
                spec matrix below is still populated from the extracted attributes.
              </div>
            )}
            <CitationList citations={compare.data.citations} />
          </div>

          <div className="card compare-scroll">
            <table className="compare-table">
              <thead>
                <tr>
                  <th>Spec</th>
                  {compare.data.products.map((p) => (
                    <th key={p.id}>
                      {p.name}
                      {p.brand && <div className="muted">{p.brand}</div>}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {compare.data.matrix.map((row) => {
                  const distinct = new Set(
                    row.values.map((v) => (v === null || v === undefined ? "" : JSON.stringify(v))),
                  );
                  const differs = distinct.size > 1;
                  return (
                    <tr key={row.key}>
                      <th>{row.key.replace(/_/g, " ")}</th>
                      {row.values.map((value, i) => (
                        <td key={i} className={differs ? "diff" : undefined}>
                          {value === null || value === undefined ? "—" : String(value)}
                        </td>
                      ))}
                    </tr>
                  );
                })}
                {compare.data.matrix.length === 0 && (
                  <tr>
                    <td colSpan={compare.data.products.length + 1} className="empty">
                      No extracted attributes to compare yet.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </>
      )}
    </div>
  );
}
