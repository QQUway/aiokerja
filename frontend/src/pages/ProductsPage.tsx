import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useCreateProduct, useDeleteProduct, useProducts } from "../api/products";
import { DEVICE_TYPES, type DeviceType, type Product } from "../api/types";
import ConfirmDialog from "../components/ConfirmDialog";

function deviceTypeLabel(value: string | null) {
  return value ? value.replace(/_/g, " ") : "—";
}

export default function ProductsPage() {
  const navigate = useNavigate();
  const products = useProducts();
  const createProduct = useCreateProduct();
  const removeProduct = useDeleteProduct();
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [category, setCategory] = useState("");
  const [brand, setBrand] = useState("");
  const [model, setModel] = useState("");
  const [deviceType, setDeviceType] = useState<DeviceType | "">("");
  const [attributes, setAttributes] = useState("{}");
  const [confirmDelete, setConfirmDelete] = useState<Product | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());

  const selectedList = useMemo(() => Array.from(selected), [selected]);

  const toggleSelected = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  return (
    <div>
      <div className="page-header">
        <h2>Products</h2>
        <div style={{ display: "flex", gap: 6 }}>
          <button
            className="primary"
            disabled={selectedList.length < 2}
            title={
              selectedList.length < 2
                ? "Select at least 2 products to compare"
                : `Compare ${selectedList.length} products`
            }
            onClick={() => navigate(`/products/compare?ids=${selectedList.join(",")}`)}
          >
            Compare selected ({selectedList.length})
          </button>
          <button onClick={() => setCreating((v) => !v)}>
            {creating ? "Close" : "New product"}
          </button>
        </div>
      </div>

      <p className="muted">
        Datasheet catalog for AIDC hardware (RFID readers/antennas, handheld computers,
        barcode/RFID printers, scanners). Extract specs from a datasheet on its document page,
        or add a product manually below. Select two or more to compare.
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
              {
                name,
                category: category || null,
                brand: brand || null,
                model: model || null,
                device_type: deviceType || null,
                attributes: parsed,
              },
              {
                onSuccess: () => {
                  setName("");
                  setCategory("");
                  setBrand("");
                  setModel("");
                  setDeviceType("");
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
            <label>Device type</label>
            <select value={deviceType} onChange={(e) => setDeviceType(e.target.value as DeviceType | "")}>
              <option value="">—</option>
              {DEVICE_TYPES.map((dt) => (
                <option key={dt} value={dt}>
                  {deviceTypeLabel(dt)}
                </option>
              ))}
            </select>
          </div>
          <div className="form-row">
            <label>Brand</label>
            <input value={brand} onChange={(e) => setBrand(e.target.value)} />
          </div>
          <div className="form-row">
            <label>Model</label>
            <input value={model} onChange={(e) => setModel(e.target.value)} />
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
              <th />
              <th>Name</th>
              <th>Brand</th>
              <th>Model</th>
              <th>Device type</th>
              <th>Specs</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {products.data.map((product) => (
              <tr key={product.id}>
                <td style={{ paddingLeft: 24 }}>
                  <input
                    id={`select-${product.id}`}
                    type="checkbox"
                    style={{ width: "auto" }}
                    checked={selected.has(product.id)}
                    onChange={() => toggleSelected(product.id)}
                  />
                  <label htmlFor={`select-${product.id}`} className="checkbox-only-label">
                    <span className="sr-only">Select {product.name}</span>
                  </label>
                </td>
                <td>{product.name}</td>
                <td>{product.brand ?? "—"}</td>
                <td>{product.model ?? "—"}</td>
                <td>{deviceTypeLabel(product.device_type)}</td>
                <td>{Object.keys(product.attributes ?? {}).length} field(s)</td>
                <td>
                  <button className="danger" onClick={() => setConfirmDelete(product)}>
                    Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {confirmDelete && (
        <ConfirmDialog
          title="Delete product"
          message={`Delete product "${confirmDelete.name}"?`}
          confirmLabel="Delete"
          onConfirm={() => {
            removeProduct.mutate(confirmDelete.id);
            setConfirmDelete(null);
          }}
          onCancel={() => setConfirmDelete(null)}
        />
      )}
    </div>
  );
}
