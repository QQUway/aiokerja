import type { TaskListItem } from "../api/types";
import PriorityBadge from "./PriorityBadge";
import StatusBadge from "./StatusBadge";

interface Props {
  task: TaskListItem;
  onClick?: () => void;
  onStatusChange?: (status: TaskListItem["status"]) => void;
}

export default function TaskCard({ task, onClick, onStatusChange }: Props) {
  return (
    <div className="task-card">
      <div className="title" onClick={onClick} style={onClick ? { cursor: "pointer" } : undefined}>
        {task.title}
      </div>
      <div style={{ display: "flex", gap: 6, flexWrap: "wrap", alignItems: "center" }}>
        <PriorityBadge priority={task.priority} />
        <StatusBadge status={task.status} />
        {task.due_date && (
          <span className="muted" style={{ fontSize: "0.75rem" }}>
            due {new Date(task.due_date).toLocaleDateString()}
          </span>
        )}
      </div>
      {task.tags.length > 0 && (
        <div style={{ marginTop: 6 }}>
          {task.tags.map((tag) => (
            <span
              key={tag.id}
              className="tag-chip"
              style={{ backgroundColor: tag.color }}
            >
              {tag.name}
            </span>
          ))}
        </div>
      )}
      {onStatusChange && (
        <select
          value={task.status}
          onChange={(e) => onStatusChange(e.target.value as TaskListItem["status"])}
          style={{ marginTop: 6 }}
        >
          <option value="todo">Todo</option>
          <option value="in_progress">In progress</option>
          <option value="done">Done</option>
          <option value="cancelled">Cancelled</option>
        </select>
      )}
    </div>
  );
}