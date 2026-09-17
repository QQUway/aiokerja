import type { Priority } from "../api/types";

const labels: Record<Priority, string> = {
  low: "Low",
  medium: "Medium",
  high: "High",
  urgent: "Urgent",
};

export default function PriorityBadge({ priority }: { priority: Priority }) {
  return <span className={`badge ${priority}`}>{labels[priority]}</span>;
}