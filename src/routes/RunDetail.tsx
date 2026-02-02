import React, { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  Box,
  Tabs,
  Card,
  Text,
  Badge,
  Stack,
  Group,
  Anchor,
  Code,
  Button,
} from "@mantine/core";
import { IconArrowLeft, IconMessage, IconTool } from "@tabler/icons-react";
import { PageHeader } from "../ui";
import ToolCallList from "../components/ToolCallList";
import { getProject, getTask, listMessages, listToolCalls } from "../lib/api";
import type { Message, Project, Task } from "../lib/types";

interface ToolCall {
  id: string;
  name: string;
  args_json: string;
  result_json: string;
  created_at: string;
}

export default function RunDetail() {
  const { id: projectId, taskId, runId } = useParams<{ id: string; taskId: string; runId: string }>();
  const [project, setProject] = useState<Project | null>(null);
  const [task, setTask] = useState<Task | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [toolCalls, setToolCalls] = useState<ToolCall[]>([]);

  useEffect(() => {
    if (!projectId || !taskId || !runId) return;
    (async () => {
      setProject(await getProject(projectId));
      setTask(await getTask(taskId));
      setMessages(await listMessages(runId));
      setToolCalls(await listToolCalls(runId));
    })();
  }, [projectId, taskId, runId]);

  if (!project || !task) return <Text>Loading…</Text>;

  const getRoleColor = (role: Message["role"]) => {
    switch (role) {
      case "user": return "blue";
      case "assistant": return "green";
      case "tool": return "gray";
      default: return "gray";
    }
  };

  return (
    <Box>
      <PageHeader
        title="Run Log"
        subtitle={
          <Group gap="xs">
            <Anchor component={Link} to={`/projects/${project.id}`}>
              {project.name}
            </Anchor>
            <Text c="dimmed">/</Text>
            <Anchor component={Link} to={`/projects/${project.id}/tasks/${task.id}`}>
              {task.title}
            </Anchor>
          </Group>
        }
        rightActions={
          <Button component={Link} to={`/projects/${project.id}/tasks/${task.id}`} variant="light" leftSection={<IconArrowLeft size={16} />}>
            Back to Task
          </Button>
        }
      />

      <Tabs defaultValue="messages">
        <Tabs.List>
          <Tabs.Tab value="messages" leftSection={<IconMessage size={16} />}>
            Messages ({messages.length})
          </Tabs.Tab>
          <Tabs.Tab value="tools" leftSection={<IconTool size={16} />}>
            Tool Calls ({toolCalls.length})
          </Tabs.Tab>
        </Tabs.List>

        <Tabs.Panel value="messages" pt="md">
          <Stack>
            {messages.map((m) => (
              <Card key={m.id} withBorder shadow="sm" radius="md">
                <Group mb="xs">
                  <Badge color={getRoleColor(m.role)}>{m.role.toUpperCase()}</Badge>
                  <Text size="xs" c="dimmed">
                    {new Date(m.created_at).toLocaleString()}
                  </Text>
                </Group>
                <Code block styles={{ root: { whiteSpace: "pre-wrap" } }}>
                  {m.content}
                </Code>
              </Card>
            ))}
            {messages.length === 0 && (
              <Card withBorder padding="md" radius="md" style={{ borderStyle: "dashed" }}>
                <Text c="dimmed" ta="center">
                  No messages yet for this run.
                </Text>
              </Card>
            )}
          </Stack>
        </Tabs.Panel>

        <Tabs.Panel value="tools" pt="md">
          <ToolCallList toolCalls={toolCalls} />
        </Tabs.Panel>
      </Tabs>
    </Box>
  );
}
