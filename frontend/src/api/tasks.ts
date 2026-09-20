import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, toQuery } from "./client";
import type { Task, TaskDetail, TaskPage, TaskStatus, Priority, TaskSummary } from "./types";

export interface TaskFilter {
  status?: string;
  priority?: string;
  project_id?: string;
  tag_id?: string;
  search?: string;
  include_subtasks?: boolean;
  page?: number;
  limit?: number;
}

export interface NewTask {
  title: string;
  description?: string | null;
  status?: TaskStatus;
  priority?: Priority;
  due_date?: string | null;
  scheduled_start?: string | null;
  scheduled_end?: string | null;
  recurrence_rule?: string | null;
  project_id?: string | null;
  parent_task_id?: string | null;
  tag_ids?: string[];
}

export interface UpdateTask {
  title?: string;
  description?: string | null;
  status?: TaskStatus;
  priority?: Priority;
  due_date?: string | null;
  scheduled_start?: string | null;
  scheduled_end?: string | null;
  project_id?: string | null;
  tag_ids?: string[];
}

export function taskKeys(filter: TaskFilter) {
  return ["tasks", filter] as const;
}

export function useTasks(filter: TaskFilter = {}) {
  return useQuery({
    queryKey: taskKeys(filter),
    queryFn: () => api.get<TaskPage>(`/tasks${toQuery(filter)}`),
  });
}

export function useTask(id: string | undefined) {
  return useQuery({
    queryKey: ["tasks", "detail", id],
    queryFn: () => api.get<TaskDetail>(`/tasks/${id}`),
    enabled: !!id,
  });
}

export function useTaskSummaries() {
  return useQuery({
    queryKey: ["tasks", "summary"],
    queryFn: () => api.get<TaskSummary>(`/tasks/views/summary`),
  });
}

export function useCreateTask() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: NewTask) => api.post<TaskDetail>("/tasks", body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["tasks"] }),
  });
}

export function useUpdateTask() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: UpdateTask }) =>
      api.patch<TaskDetail>(`/tasks/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["tasks"] }),
  });
}

export function useDeleteTask() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.delete<{ deleted: boolean }>(`/tasks/${id}`),
    onMutate: async (id: string) => {
      await qc.cancelQueries({ queryKey: ["tasks"] });
      const previousEntries = qc.getQueriesData<TaskPage>({
        queryKey: ["tasks"],
        predicate: (query) => Array.isArray((query.state.data as TaskPage | undefined)?.items),
      });
      for (const [key, data] of previousEntries) {
        if (data) {
          qc.setQueryData<TaskPage>(key, {
            ...data,
            items: data.items.filter((t) => t.id !== id),
            total: Math.max(0, data.total - 1),
          });
        }
      }
      return { previousEntries };
    },
    onError: (_err, _id, context) => {
      context?.previousEntries?.forEach(([key, data]) => qc.setQueryData(key, data));
    },
    onSettled: () => qc.invalidateQueries({ queryKey: ["tasks"] }),
  });
}

export function useMaterializeEvent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.post(`/tasks/${id}/calendar-event`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["calendar"] });
      qc.invalidateQueries({ queryKey: ["events"] });
    },
  });
}

export function useSearchTasks() {
  return useMutation({
    mutationFn: (query: string) =>
      api.get<{ tasks: Task[] }>(`/tasks${toQuery({ search: query })}`),
  });
}