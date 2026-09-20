import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./client";
import type { ComparisonResult, DeviceType, Product } from "./types";

export interface NewProduct {
  name: string;
  category?: string | null;
  attributes?: Record<string, unknown>;
  brand?: string | null;
  model?: string | null;
  device_type?: DeviceType | null;
}

export function useProducts() {
  return useQuery({
    queryKey: ["products"],
    queryFn: () => api.get<Product[]>("/products"),
  });
}

export function useExtractDatasheet() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ documentId, deviceType }: { documentId: string; deviceType?: DeviceType }) =>
      api.post<Product>(`/documents/${documentId}/extract-datasheet`, {
        device_type: deviceType,
      }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["products"] }),
  });
}

export function useCompareProducts() {
  return useMutation({
    mutationFn: ({ productIds, question }: { productIds: string[]; question?: string }) =>
      api.post<ComparisonResult>("/products/compare", {
        product_ids: productIds,
        question,
      }),
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
    onMutate: async (id: string) => {
      await qc.cancelQueries({ queryKey: ["products"] });
      const previous = qc.getQueryData<Product[]>(["products"]);
      qc.setQueryData<Product[]>(["products"], (old) => old?.filter((p) => p.id !== id));
      return { previous };
    },
    onError: (_err, _id, context) => {
      if (context?.previous) qc.setQueryData(["products"], context.previous);
    },
    onSettled: () => qc.invalidateQueries({ queryKey: ["products"] }),
  });
}