import { useQuery } from "@tanstack/react-query";
import { api, toQuery } from "./client";
import type { SearchResults } from "./types";

export function useSearch(query: string) {
  return useQuery({
    queryKey: ["search", query],
    queryFn: () => api.get<SearchResults>(`/search${toQuery({ q: query })}`),
    enabled: query.trim().length > 1,
  });
}