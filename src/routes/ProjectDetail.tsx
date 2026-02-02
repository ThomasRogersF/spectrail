import React, { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  Card,
  Text,
  Button,
  TextInput,
  Group,
  Stack,
  Badge,
  Box,
  SegmentedControl,
} from "@mantine/core";
import { IconArrowLeft, IconPlus } from "@tabler/icons-react";
import { PageHeader } from "../ui";
import { createTask, getProject, listTasks, touchProject } from "../lib/api";
import type { Project, Task } from "../lib/types";

export default function ProjectDetail() {
  const { id } = useParams<{ id: string }>();
  const [project, setProject] = useState<Project | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [title, setTitle] = useState("");
  const [taskMode, setTaskMode] = useState<Task["mode"]>("plan");

  useEffect(() => {
    if (!id) return;
    (async () => {
      await touchProject(id);
      setProject(await getProject(id));
      setTasks(await listTasks(id));
    })();
  }, [id]);

  async function onCreateTask() {
    if (!id) return;
    const t = await createTask(id, title.trim() || "New Task", taskMode);
    setTitle("");
    setTasks([t, ...tasks]);
  }

  if (!project) return <Text>Loading…</Text>;

  const getStatusColor = (status: Task["status"]) => {
    switch (status) {
      case "done": return "green";
      case "active": return "blue";
      case "draft": return "gray";
      case "archived": return "red";
      default: return "gray";
    }
  };

  return (
    <Box>
      <PageHeader
        title={project.name}
        subtitle={project.repo_path}
        rightActions={
          <Button component={Link} to="/projects" variant="light" leftSection={<IconArrowLeft size={16} />}>
            Back
          </Button>
        }
      />

      <Card withBorder shadow="sm" radius="md" mb="lg">
        <Group align="flex-end">
          <TextInput
            label="Task Title"
            placeholder="e.g., Add OAuth login"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            style={{ flex: 1 }}
          />
          <SegmentedControl
            value={taskMode}
            onChange={(value) => setTaskMode(value as Task["mode"])}
            data={[
              { label: "Plan", value: "plan" },
              { label: "Phases", value: "phases" },
            ]}
          />
          <Button onClick={onCreateTask} leftSection={<IconPlus size={16} />}>
            Create Task
          </Button>
        </Group>
      </Card>

      <Stack>
        {tasks.map((t) => (
          <Card
            key={t.id}
            component={Link}
            to={`/projects/${project.id}/tasks/${t.id}`}
            withBorder
            shadow="sm"
            radius="md"
            padding="md"
            style={{ textDecoration: "none", color: "inherit" }}
          >
            <Group justify="space-between" align="center">
              <div>
                <Text fw={700} size="lg">{t.title}</Text>
                <Text size="sm" c="dimmed">
                  Created {new Date(t.created_at).toLocaleDateString()}
                </Text>
              </div>
              <Group>
                <Badge color="blue" variant="light">{t.mode.toUpperCase()}</Badge>
                <Badge color={getStatusColor(t.status)} variant="light">{t.status}</Badge>
              </Group>
            </Group>
          </Card>
        ))}
        {tasks.length === 0 && (
          <Card withBorder padding="md" radius="md" style={{ borderStyle: "dashed" }}>
            <Text c="dimmed" ta="center">
              No tasks yet. Create one above to get started.
            </Text>
          </Card>
        )}
      </Stack>
    </Box>
  );
}
