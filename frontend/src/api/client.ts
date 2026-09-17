const BASE = "/api";

export class ApiError extends Error {
  code: string;
  status: number;

  constructor(message: string, code = "error", status = 0) {
    super(message);
    this.code = code;
    this.status = status;
  }
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const isFormData = options?.body instanceof FormData;
  const response = await fetch(`${BASE}${path}`, {
    headers: options?.body && !isFormData ? { "Content-Type": "application/json" } : undefined,
    ...options,
  });

  if (!response.ok) {
    let message = `request failed (${response.status})`;
    let code = "error";
    try {
      const body = await response.json();
      if (body?.error?.message) message = body.error.message;
      if (body?.error?.code) code = body.error.code;
    } catch {
      // non-JSON error body
    }
    throw new ApiError(message, code, response.status);
  }

  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

export const api = {
  get: <T>(path: string) => request<T>(path),

  post: <T>(path: string, body?: unknown) =>
    request<T>(path, {
      method: "POST",
      body: body === undefined ? undefined : JSON.stringify(body),
    }),

  patch: <T>(path: string, body?: unknown) =>
    request<T>(path, {
      method: "PATCH",
      body: body === undefined ? undefined : JSON.stringify(body),
    }),

  delete: <T>(path: string) =>
    request<T>(path, { method: "DELETE" }),

  postFile<T>(path: string, file: File): Promise<T> {
    const form = new FormData();
    form.append("file", file);
    return request<T>(path, { method: "POST", body: form });
  },
};

export function toQuery(params: object): string {
  const parts = Object.entries(params as Record<string, unknown>)
    .filter(([, v]) => v !== null && v !== undefined && v !== "")
    .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`);
  return parts.length ? `?${parts.join("&")}` : "";
}