import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./client";
import type { Tag } from "./types";

export function useTags() {
  return useQuery({
    queryKey: ["tags"],
    queryFn: () => api.get<Tag[]>("/tags"),
  });
}

export function useCreateTag() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: { name: string; color?: string }) =>
      api.post<Tag>("/tags", body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["tags"] }),
  });
}