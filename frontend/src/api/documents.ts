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
    onSuccess: () => qc.invalidateQueries({ queryKey: ["documents"] }),
  });
}