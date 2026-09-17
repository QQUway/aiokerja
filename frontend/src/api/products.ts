import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./client";
import type { Product } from "./types";

export interface NewProduct {
  name: string;
  category?: string | null;
  attributes?: Record<string, unknown>;
}

export function useProducts() {
  return useQuery({
    queryKey: ["products"],
    queryFn: () => api.get<Product[]>("/products"),
  });
}

export function useCreateProduct() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: NewProduct) => api.post<Product>("/products", body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["products"] }),
  });
}

export function useDeleteProduct() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.delete<{ deleted: boolean }>(`/products/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["products"] }),
  });
}