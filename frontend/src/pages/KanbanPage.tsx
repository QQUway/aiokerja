import { useTasks, useUpdateTask } from "../api/tasks";
import type { TaskStatus } from "../api/types";
import TaskCard from "../components/TaskCard";

const columns: { status: TaskStatus; label: string }[] = [
  { status: "todo", label: "Todo" },
  { status: "in_progress", label: "In progress" },
  { status: "done", label: "Done" },
];

export default function KanbanPage() {
  const { data, isLoading, error } = useTasks({ limit: 500 });
  const updateTask = useUpdateTask();

  if (isLoading) return <div className="empty">Loading board…</div>;
  if (error) return <div className="error">{(error as Error).message}</div>;

  const tasks = data?.items ?? [];

  return (
    <div>
      <div className="page-header">
        <h2>Kanban</h2>
      </div>
      <div className="kanban">
        {columns.map((column) => {
          const items = tasks.filter((task) => task.status === column.status);
          return (
            <div className="kanban-column" key={column.status}>
              <h3>
                {column.label} ({items.length})
              </h3>
              {items.length === 0 && <div className="empty">Empty</div>}
              {items.map((task) => (
                <TaskCard
                  key={task.id}
                  task={task}
                  onStatusChange={(status) =>
                    updateTask.mutate({ id: task.id, body: { status } })
                  }
                />
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}