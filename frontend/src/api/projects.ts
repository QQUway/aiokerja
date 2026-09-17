import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./client";
import type { Project } from "./types";

export interface NewProject {
  name: string;
  description?: string | null;
  color?: string;
}

export function useProjects() {
  return useQuery({
    queryKey: ["projects"],
    queryFn: () => api.get<Project[]>("/projects"),
  });
}

export function useProject(id: string | undefined) {
  return useQuery({
    queryKey: ["projects", id],
    queryFn: () =>
      api.get<{ project: Project; task_count: number }>(`/projects/${id}`),
    enabled: !!id,
  });
}

export function useCreateProject() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: NewProject) => api.post<Project>("/projects", body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["projects"] }),
  });
}

export function useUpdateProject() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: Partial<NewProject> }) =>
      api.patch<Project>(`/projects/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["projects"] }),
  });
}

export function useDeleteProject() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, cascade }: { id: string; cascade?: boolean }) =>
      api.delete<{ deleted: boolean }>(
        `/projects/${id}${cascade ? "?cascade=true" : ""}`,
      ),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["projects"] }),
  });
}