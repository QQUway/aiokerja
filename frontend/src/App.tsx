import { useState } from "react";
import { NavLink, Outlet, Route, Routes } from "react-router-dom";
import LoginPage, { isAuthenticated, logout } from "./pages/LoginPage";
import Dashboard from "./pages/Dashboard";
import TasksPage from "./pages/TasksPage";
import KanbanPage from "./pages/KanbanPage";
import ProjectsPage from "./pages/ProjectsPage";
import ProjectDetailPage from "./pages/ProjectDetailPage";
import CalendarPage from "./pages/CalendarPage";
import DocumentsPage from "./pages/DocumentsPage";
import DocumentDetailPage from "./pages/DocumentDetailPage";
import ChatPage from "./pages/ChatPage";
import SearchPage from "./pages/SearchPage";
import ProductsPage from "./pages/ProductsPage";
import ProductComparePage from "./pages/ProductComparePage";

const nav = [
  { to: "/", label: "Dashboard" },
  { to: "/tasks", label: "Tasks" },
  { to: "/tasks/kanban", label: "Kanban" },
  { to: "/projects", label: "Projects" },
  { to: "/calendar", label: "Calendar" },
  { to: "/documents", label: "Documents" },
  { to: "/chat", label: "Chat" },
  { to: "/search", label: "Search" },
  { to: "/products", label: "Products" },
];

function Layout() {
  return (
    <div className="desktop">
      <div className="window app-window">
        <div className="title-bar">
          <div className="title-bar-text">Work Assistant</div>
          <div className="title-bar-controls">
            <button aria-label="Minimize" />
            <button aria-label="Maximize" />
            <button aria-label="Close" onClick={logout} title="Log off" />
          </div>
        </div>
        <div className="window-body app-window-body">
          <div className="layout">
            <aside className="sidebar">
              <h1 className="brand">Work Assistant</h1>
              <nav>
                {nav.map((item) => (
                  <NavLink
                    key={item.to}
                    to={item.to}
                    end={item.to === "/"}
                    className={({ isActive }) => (isActive ? "nav-link active" : "nav-link")}
                  >
                    {item.label}
                  </NavLink>
                ))}
              </nav>
            </aside>
            <main className="content">
              <Outlet />
            </main>
          </div>
        </div>
      </div>
    </div>
  );
}

export default function App() {
  const [authed, setAuthed] = useState(isAuthenticated());

  if (!authed) {
    return <LoginPage onSuccess={() => setAuthed(true)} />;
  }

  return (
    <Routes>
      <Route element={<Layout />}>
        <Route path="/" element={<Dashboard />} />
        <Route path="/tasks" element={<TasksPage />} />
        <Route path="/tasks/kanban" element={<KanbanPage />} />
        <Route path="/projects" element={<ProjectsPage />} />
        <Route path="/projects/:id" element={<ProjectDetailPage />} />
        <Route path="/calendar" element={<CalendarPage />} />
        <Route path="/documents" element={<DocumentsPage />} />
        <Route path="/documents/:id" element={<DocumentDetailPage />} />
        <Route path="/chat" element={<ChatPage />} />
        <Route path="/search" element={<SearchPage />} />
        <Route path="/products" element={<ProductsPage />} />
        <Route path="/products/compare" element={<ProductComparePage />} />
      </Route>
    </Routes>
  );
}