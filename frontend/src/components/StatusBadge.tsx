import type { TaskStatus } from "../api/types";

const labels: Record<TaskStatus, string> = {
  todo: "Todo",
  in_progress: "In progress",
  done: "Done",
  cancelled: "Cancelled",
};

export default function StatusBadge({ status }: { status: TaskStatus }) {
  return <span className={`badge ${status}`}>{labels[status]}</span>;
}