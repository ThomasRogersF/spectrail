import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Card,
  Text,
  Button,
  TextInput,
  Group,
  Stack,
  Badge,
  Box,
} from "@mantine/core";
import { IconFolder } from "@tabler/icons-react";
import { PageHeader } from "../ui";
import { createProject, dbHealth, listProjects } from "../lib/api";
import type { Project } from "../lib/types";

export default function Projects() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [health, setHealth] = useState<{ ok: boolean; path: string } | null>(null);
  const [name, setName] = useState("");

  async function refresh() {
    setProjects(await listProjects());
  }

  useEffect(() => {
    (async () => {
      setHealth(await dbHealth());
      await refresh();
    })();
  }, []);

  async function onPickRepo() {
    const selected = await open({ directory: true, multiple: false, title: "Select a repo folder" });
    if (!selected) return;
    const repoPath = Array.isArray(selected) ? selected[0] : selected;
    const projName = name.trim() || repoPath.split(/[\\/]/).filter(Boolean).slice(-1)[0] || "Untitled Project";
    const p = await createProject(projName, repoPath);
    setName("");
    setProjects([p, ...projects]);
  }

  return (
    <Box>
      <PageHeader
        title="Projects"
        subtitle={`DB: ${health?.ok ? "OK" : "…"} ${health?.path ? `(${health.path})` : ""}`}
      />

      <Group align="flex-end" mb="lg">
        <TextInput
          label="Project Name"
          placeholder="Optional project name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          style={{ flex: 1 }}
        />
        <Button onClick={onPickRepo} leftSection={<IconFolder size={16} />}>
          Add from folder…
        </Button>
      </Group>

      <Stack>
        {projects.map((p) => (
          <Card
            key={p.id}
            component={Link}
            to={`/projects/${p.id}`}
            withBorder
            shadow="sm"
            radius="md"
            padding="md"
            style={{ textDecoration: "none", color: "inherit" }}
          >
            <Group justify="space-between" align="center">
              <div>
                <Text fw={700} size="lg">{p.name}</Text>
                <Text size="sm" c="dimmed">{p.repo_path}</Text>
              </div>
              <Badge variant="light">{new Date(p.created_at).toLocaleDateString()}</Badge>
            </Group>
          </Card>
        ))}
        {projects.length === 0 && (
          <Card withBorder padding="md" radius="md" style={{ borderStyle: "dashed" }}>
            <Text c="dimmed" ta="center">
              No projects yet. Click "Add from folder…" to get started.
            </Text>
          </Card>
        )}
      </Stack>
    </Box>
  );
}
