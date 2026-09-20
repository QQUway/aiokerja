import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, toQuery } from "./client";
import type { Document, DocumentChunk } from "./types";

export interface DocumentQuery {
  status?: string;
  doc_type?: string;
  search?: string;
}

export function useDocuments(query: DocumentQuery = {}) {
  return useQuery({
    queryKey: ["documents", query],
    queryFn: () =>
      api.get<Document[]>(`/documents${toQuery(query)}`),
  });
}

export function useDocument(id: string | undefined) {
  return useQuery({
    queryKey: ["documents", "detail", id],
    queryFn: () =>
      api.get<{ document: Document; chunk_count: number; raw_text_length: number }>(
        `/documents/${id}`,
      ),
    enabled: !!id,
  });
}

export function useDocumentChunks(id: string | undefined) {
  return useQuery({
    queryKey: ["documents", "chunks", id],
    queryFn: () => api.get<DocumentChunk[]>(`/documents/${id}/chunks`),
    enabled: !!id,
  });
}

export function useUploadDocument() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (file: File) => api.postFile<Document>("/documents", file),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["documents"] }),
  });
}

export function useReindexDocument() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.post(`/documents/${id}/reindex`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["documents"] });
    },
  });
}

export function useDeleteDocument() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.delete<{ deleted: boolean }>(`/documents/${id}`),
    onMutate: async (id: string) => {
      await qc.cancelQueries({ queryKey: ["documents"] });
      const previousEntries = qc.getQueriesData<Document[]>({
        queryKey: ["documents"],
        predicate: (query) => query.queryKey.length === 2,
      });
      for (const [key, data] of previousEntries) {
        if (Array.isArray(data)) {
          qc.setQueryData<Document[]>(key, data.filter((d) => d.id !== id));
        }
      }
      return { previousEntries };
    },
    onError: (_err, _id, context) => {
      context?.previousEntries?.forEach(([key, data]) => qc.setQueryData(key, data));
    },
    onSettled: () => qc.invalidateQueries({ queryKey: ["documents"] }),
  });
}