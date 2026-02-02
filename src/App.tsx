import React from "react";
import { Route, Routes } from "react-router-dom";
import { AppLayout } from "./ui";
import Projects from "./routes/Projects";
import ProjectDetail from "./routes/ProjectDetail";
import TaskDetail from "./routes/TaskDetail";
import RunDetail from "./routes/RunDetail";
import Settings from "./routes/Settings";

export default function App() {
  return (
    <Routes>
      <Route element={<AppLayout />}>
        <Route path="/" element={<Projects />} />
        <Route path="/projects" element={<Projects />} />
        <Route path="/projects/:id" element={<ProjectDetail />} />
        <Route path="/projects/:id/tasks/:taskId" element={<TaskDetail />} />
        <Route path="/projects/:id/tasks/:taskId/runs/:runId" element={<RunDetail />} />
        <Route path="/settings" element={<Settings />} />
      </Route>
    </Routes>
  );
}
