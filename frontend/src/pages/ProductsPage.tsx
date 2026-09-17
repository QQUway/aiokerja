import { useState } from "react";
import { useCreateProduct, useDeleteProduct, useProducts } from "../api/products";

export default function ProductsPage() {
  const products = useProducts();
  const createProduct = useCreateProduct();
  const removeProduct = useDeleteProduct();
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [category, setCategory] = useState("");
  const [attributes, setAttributes] = useState("{}");

  return (
    <div>
      <div className="page-header">
        <h2>Products</h2>
        <button className="primary" onClick={() => setCreating((v) => !v)}>
          {creating ? "Close" : "New product"}
        </button>
      </div>

      <p className="muted">
        Category-agnostic product catalog: attributes are free-form JSON, so a microcontroller
        and a laptop both fit.
      </p>

      {creating && (
        <form
          className="card"
          onSubmit={(e) => {
            e.preventDefault();
            let parsed: Record<string, unknown>;
            try {
              parsed = JSON.parse(attributes || "{}");
            } catch {
              parsed = {};
            }
            createProduct.mutate(
              { name, category: category || null, attributes: parsed },
              {
                onSuccess: () => {
                  setName("");
                  setCategory("");
                  setAttributes("{}");
                  setCreating(false);
                },
              },
            );
          }}
        >
          <div className="form-row">
            <label>Name *</label>
            <input value={name} onChange={(e) => setName(e.target.value)} required autoFocus />
          </div>
          <div className="form-row">
            <label>Category</label>
            <input value={category} onChange={(e) => setCategory(e.target.value)} />
          </div>
          <div className="form-row">
            <label>Attributes (JSON)</label>
            <textarea
              value={attributes}
              onChange={(e) => setAttributes(e.target.value)}
              spellCheck={false}
            />
          </div>
          <div className="form-actions">
            <button type="button" onClick={() => setCreating(false)}>
              Cancel
            </button>
            <button type="submit" className="primary" disabled={createProduct.isPending || !name.trim()}>
              Create
            </button>
          </div>
        </form>
      )}

      {createProduct.error && <div className="error">{(createProduct.error as Error).message}</div>}
      {removeProduct.error && <div className="error">{(removeProduct.error as Error).message}</div>}

      {products.isLoading && <div className="empty">Loading…</div>}
      {products.error && <div className="error">{(products.error as Error).message}</div>}

      {products.data && products.data.length === 0 && !products.isLoading && (
        <div className="empty">No products yet.</div>
      )}

      {products.data && products.data.length > 0 && (
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Category</th>
              <th>Attributes</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {products.data.map((product) => (
              <tr key={product.id}>
                <td>{product.name}</td>
                <td>{product.category ?? "—"}</td>
                <td>
                  <code>{JSON.stringify(product.attributes)}</code>
                </td>
                <td>
                  <button
                    className="danger"
                    onClick={() => {
                      if (confirm(`Delete product "${product.name}"?`)) removeProduct.mutate(product.id);
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