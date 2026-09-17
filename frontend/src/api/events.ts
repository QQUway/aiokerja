import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, toQuery } from "./client";
import type { CalendarEvent, CalendarItem } from "./types";

export interface NewEvent {
  title: string;
  description?: string | null;
  start_time?: string;
  end_time?: string;
  all_day?: boolean;
  recurrence_rule?: string | null;
  reminder_minutes?: number | null;
}

export function useCalendar(from: string, to: string) {
  return useQuery({
    queryKey: ["calendar", from, to],
    queryFn: () =>
      api.get<CalendarItem[]>(`/calendar${toQuery({ from, to })}`),
  });
}

export function useEvents(from?: string, to?: string) {
  return useQuery({
    queryKey: ["events", from, to],
    queryFn: () =>
      api.get<CalendarEvent[]>(`/events${toQuery({ from, to })}`),
  });
}

export function useCreateEvent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: NewEvent) => api.post<CalendarEvent>("/events", body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["calendar"] });
      qc.invalidateQueries({ queryKey: ["events"] });
    },
  });
}

export function useUpdateEvent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: Partial<NewEvent> }) =>
      api.patch<CalendarEvent>(`/events/${id}`, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["calendar"] });
      qc.invalidateQueries({ queryKey: ["events"] });
    },
  });
}

export function useDeleteEvent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.delete<{ deleted: boolean }>(`/events/${id}`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["calendar"] });
      qc.invalidateQueries({ queryKey: ["events"] });
    },
  });
}