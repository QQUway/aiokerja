// Types mirror the backend JSON API (snake_case, RFC3339 timestamps).

export type TaskStatus = "todo" | "in_progress" | "done" | "cancelled";
export type Priority = "low" | "medium" | "high" | "urgent";

export interface Tag {
  id: string;
  name: string;
  color: string;
  created_at: string;
}

export interface Task {
  id: string;
  title: string;
  description: string | null;
  status: TaskStatus;
  priority: Priority;
  due_date: string | null;
  scheduled_start: string | null;
  scheduled_end: string | null;
  recurrence_rule: string | null;
  project_id: string | null;
  parent_task_id: string | null;
  completed_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface TaskListItem extends Task {
  tags: Tag[];
}

export interface TaskDetail extends Task {
  tags: Tag[];
  subtasks: Task[];
}

export interface TaskPage {
  items: TaskListItem[];
  total: number;
}

export interface Project {
  id: string;
  name: string;
  description: string | null;
  color: string;
  created_at: string;
  updated_at: string;
}

export interface CalendarEvent {
  id: string;
  title: string;
  description: string | null;
  start_time: string;
  end_time: string;
  all_day: boolean;
  recurrence_rule: string | null;
  reminder_minutes: number | null;
  source_task_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface CalendarItem {
  id: string;
  kind: "event" | "task";
  title: string;
  description: string | null;
  start_time: string;
  end_time: string;
  all_day: boolean;
  recurrence_rule: string | null;
  reminder_minutes: number | null;
  source_task_id: string | null;
  task_id: string | null;
  status: TaskStatus | null;
  priority: Priority | null;
  project_id: string | null;
}

export type DocStatus = "pending" | "indexing" | "indexed" | "failed";

export interface Document {
  id: string;
  filename: string;
  title: string;
  source: string;
  doc_type: string;
  upload_date: string;
  content_hash: string | null;
  status: DocStatus;
  error: string | null;
  created_at: string;
  updated_at: string;
}

export interface DocumentChunk {
  id: string;
  document_id: string;
  chunk_index: number;
  chunk_text: string;
  token_count: number;
  created_at: string;
}

export interface Citation {
  document_id: string;
  title: string;
  chunk_text: string;
  score: number;
}

export interface Conversation {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
}

export interface Message {
  id: string;
  conversation_id: string;
  role: "user" | "assistant" | "system" | "tool";
  content: string | null;
  tool_calls: unknown;
  tool_results: unknown;
  citations: Citation[] | null;
  created_at: string;
}

export interface ToolCallTrace {
  name: string;
  args: unknown;
  result: unknown;
}

export interface ChatResponse {
  message: Message;
  tool_calls: ToolCallTrace[];
  citations: Citation[];
}

export const DEVICE_TYPES = [
  "rfid_reader",
  "rfid_antenna",
  "handheld_computer",
  "barcode_printer",
  "rfid_printer",
  "scanner",
  "other",
] as const;
export type DeviceType = (typeof DEVICE_TYPES)[number];

export interface Product {
  id: string;
  name: string;
  category: string | null;
  attributes: Record<string, unknown>;
  brand: string | null;
  model: string | null;
  device_type: DeviceType | null;
  source_document_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface ComparisonRow {
  key: string;
  values: (unknown | null)[];
}

export interface ComparisonResult {
  products: Product[];
  matrix: ComparisonRow[];
  summary: string | null;
  citations: Citation[];
}

export interface SearchResults {
  tasks: Task[];
  documents: Document[];
}

export interface TaskSummary {
  total: number;
  by_status: Record<string, number>;
  today_count: number;
  upcoming_count: number;
  overdue_count: number;
}

export interface Dashboard {
  tasks: TaskSummary;
  projects_count: number;
  documents_count: number;
  indexed_documents: number;
  upcoming_events: CalendarEvent[];
  recent_tasks: Task[];
}