import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./client";
import type { ChatResponse, Conversation, Message } from "./types";

export function useConversations() {
  return useQuery({
    queryKey: ["conversations"],
    queryFn: () => api.get<Conversation[]>("/conversations"),
  });
}

export function useConversation(id: string | undefined) {
  return useQuery({
    queryKey: ["conversations", id],
    queryFn: () =>
      api.get<{ conversation: Conversation; messages: Message[] }>(
        `/conversations/${id}`,
      ),
    enabled: !!id,
  });
}

export function useCreateConversation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (title?: string) =>
      api.post<Conversation>("/conversations", { title }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["conversations"] }),
  });
}

export function useDeleteConversation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      api.delete<{ deleted: boolean }>(`/conversations/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["conversations"] }),
  });
}

export function useSendMessage() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, content }: { id: string; content: string }) =>
      api.post<ChatResponse>(`/conversations/${id}/messages`, { content }),
    onSuccess: (_data, variables) => {
      qc.invalidateQueries({ queryKey: ["conversations", variables.id] });
      qc.invalidateQueries({ queryKey: ["conversations"] });
      // Tool calls may have changed tasks/events.
      qc.invalidateQueries({ queryKey: ["tasks"] });
      qc.invalidateQueries({ queryKey: ["events"] });
      qc.invalidateQueries({ queryKey: ["calendar"] });
    },
  });
}