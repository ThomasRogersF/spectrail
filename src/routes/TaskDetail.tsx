import React, { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  Box,
  Tabs,
  Card,
  Text,
  Button,
  Textarea,
  Group,
  Stack,
  Badge,
  Alert,
  Checkbox,
  LoadingOverlay,
  Code,
  Anchor,
} from "@mantine/core";
import { IconArrowLeft, IconRobot, IconSearch, IconList, IconAlertCircle } from "@tabler/icons-react";
import { PageHeader, CopyAction } from "../ui";
import {
  createRun,
  generatePlan,
  getProject,
  getTask,
  listArtifacts,
  listRuns,
  upsertArtifact,
  verifyTask,
} from "../lib/api";
import type { Artifact, Project, Run, Task } from "../lib/types";

export default function TaskDetail() {
  const { id: projectId, taskId } = useParams<{ id: string; taskId: string }>();
  const [project, setProject] = useState<Project | null>(null);
  const [task, setTask] = useState<Task | null>(null);
  const [runs, setRuns] = useState<Run[]>([]);
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [draftPlan, setDraftPlan] = useState("");
  const [isGeneratingPlan, setIsGeneratingPlan] = useState(false);
  const [lastPlanRunId, setLastPlanRunId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Sprint 5: Verify Mode state
  const [isVerifying, setIsVerifying] = useState(false);
  const [verifyError, setVerifyError] = useState<string | null>(null);
  const [lastVerifyRunId, setLastVerifyRunId] = useState<string | null>(null);
  const [verifyOptions, setVerifyOptions] = useState({
    run_tests: true,
    run_lint: false,
    run_build: false,
    staged: false,
  });

  const planArtifact = useMemo(
    () => artifacts.find((a) => a.kind === "plan_md") ?? null,
    [artifacts]
  );

  const verifyArtifact = useMemo(
    () => artifacts.find((a) => a.kind === "verification_report") ?? null,
    [artifacts]
  );

  useEffect(() => {
    if (!projectId || !taskId) return;
    loadData();
  }, [projectId, taskId]);

  async function loadData() {
    if (!projectId || !taskId) return;
    setProject(await getProject(projectId));
    setTask(await getTask(taskId));
    setRuns(await listRuns(taskId));
    setArtifacts(await listArtifacts(taskId));
  }

  async function onCreateRun(runType: Run["run_type"]) {
    if (!taskId) return;
    const r = await createRun(taskId, runType);
    setRuns([r, ...runs]);
  }

  async function onSavePlan() {
    if (!taskId) return;
    const saved = await upsertArtifact(taskId, null, "plan_md", draftPlan.trim());
    setArtifacts([saved, ...artifacts.filter((a) => a.id !== saved.id)]);
  }

  async function handleGeneratePlan() {
    if (!projectId || !taskId) return;

    setIsGeneratingPlan(true);
    setError(null);

    try {
      const result = await generatePlan(projectId, taskId);
      setLastPlanRunId(result.run_id);
      await loadData();
    } catch (err: any) {
      console.error("Failed to generate plan:", err);
      setError(err?.toString?.() || String(err));
      await loadData();
    } finally {
      setIsGeneratingPlan(false);
    }
  }

  async function handleVerify() {
    if (!projectId || !taskId) return;

    setIsVerifying(true);
    setVerifyError(null);

    try {
      const result = await verifyTask(projectId, taskId, verifyOptions);
      setLastVerifyRunId(result.run_id);
      await loadData();
    } catch (err: any) {
      console.error("Failed to verify:", err);
      setVerifyError(err?.toString?.() || String(err));
      await loadData();
    } finally {
      setIsVerifying(false);
    }
  }

  if (!project || !task) return <Text>Loading…</Text>;

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
        title={task.title}
        subtitle={
          <Group gap="xs">
            <Anchor component={Link} to={`/projects/${project.id}`}>
              {project.name}
            </Anchor>
            <Text c="dimmed">/</Text>
            <Badge color={getStatusColor(task.status)}>{task.status}</Badge>
            <Badge color="blue" variant="light">{task.mode}</Badge>
          </Group>
        }
        rightActions={
          <Button component={Link} to={`/projects/${project.id}`} variant="light" leftSection={<IconArrowLeft size={16} />}>
            Back
          </Button>
        }
      />

      <Tabs defaultValue="plan">
        <Tabs.List>
          <Tabs.Tab value="plan" leftSection={<IconRobot size={16} />}>
            Plan
          </Tabs.Tab>
          <Tabs.Tab value="verify" leftSection={<IconSearch size={16} />}>
            Verify
          </Tabs.Tab>
          <Tabs.Tab value="runs" leftSection={<IconList size={16} />}>
            Runs ({runs.length})
          </Tabs.Tab>
        </Tabs.List>

        {/* Plan Tab */}
        <Tabs.Panel value="plan" pt="md">
          <Card withBorder shadow="sm" radius="md" pos="relative">
            <LoadingOverlay visible={isGeneratingPlan} overlayProps={{ blur: 2 }} />
            
            <Group justify="space-between" mb="md">
              <Text fw={700} size="lg">Plan Artifact</Text>
              <Group>
                {planArtifact && (
                  <CopyAction text={planArtifact.content} label="Copy Plan" />
                )}
                <Button
                  onClick={handleGeneratePlan}
                  loading={isGeneratingPlan}
                  disabled={isVerifying}
                  leftSection={<IconRobot size={16} />}
                >
                  Generate Plan
                </Button>
              </Group>
            </Group>

            {error && (
              <Alert icon={<IconAlertCircle size={16} />} color="red" mb="md">
                {error}
              </Alert>
            )}

            {planArtifact ? (
              <Stack>
                <Code block styles={{ root: { maxHeight: 400, overflow: "auto" } }}>
                  {planArtifact.content}
                </Code>
                {lastPlanRunId && (
                  <Anchor
                    component={Link}
                    to={`/projects/${project.id}/tasks/${task.id}/runs/${lastPlanRunId}`}
                  >
                    View Run Details →
                  </Anchor>
                )}
              </Stack>
            ) : (
              <Text c="dimmed">
                No plan saved yet. Click "Generate Plan" to create one.
              </Text>
            )}

            <Textarea
              label="Edit Plan"
              placeholder="Paste or edit a plan here…"
              value={draftPlan}
              onChange={(e) => setDraftPlan(e.target.value)}
              minRows={6}
              mt="md"
            />
            <Group justify="flex-end" mt="md">
              <Button onClick={onSavePlan} variant="light">
                Save Plan
              </Button>
            </Group>
          </Card>
        </Tabs.Panel>

        {/* Verify Tab */}
        <Tabs.Panel value="verify" pt="md">
          <Card withBorder shadow="sm" radius="md" pos="relative">
            <LoadingOverlay visible={isVerifying} overlayProps={{ blur: 2 }} />
            
            <Group justify="space-between" mb="md">
              <Text fw={700} size="lg">Verification Report</Text>
              <Group>
                {verifyArtifact && (
                  <CopyAction text={verifyArtifact.content} label="Copy Report" />
                )}
                <Button
                  onClick={handleVerify}
                  loading={isVerifying}
                  disabled={isGeneratingPlan}
                  color="green"
                  leftSection={<IconSearch size={16} />}
                >
                  Verify
                </Button>
              </Group>
            </Group>

            <Group mb="md">
              <Checkbox
                label="Tests"
                checked={verifyOptions.run_tests}
                onChange={(e) => setVerifyOptions({ ...verifyOptions, run_tests: e.currentTarget.checked })}
                disabled={isVerifying || isGeneratingPlan}
              />
              <Checkbox
                label="Lint"
                checked={verifyOptions.run_lint}
                onChange={(e) => setVerifyOptions({ ...verifyOptions, run_lint: e.currentTarget.checked })}
                disabled={isVerifying || isGeneratingPlan}
              />
              <Checkbox
                label="Build"
                checked={verifyOptions.run_build}
                onChange={(e) => setVerifyOptions({ ...verifyOptions, run_build: e.currentTarget.checked })}
                disabled={isVerifying || isGeneratingPlan}
              />
              <Checkbox
                label="Staged only"
                checked={verifyOptions.staged}
                onChange={(e) => setVerifyOptions({ ...verifyOptions, staged: e.currentTarget.checked })}
                disabled={isVerifying || isGeneratingPlan}
              />
            </Group>

            {verifyError && (
              <Alert icon={<IconAlertCircle size={16} />} color="red" mb="md">
                {verifyError}
              </Alert>
            )}

            {verifyArtifact ? (
              <Stack>
                <Code block styles={{ root: { maxHeight: 400, overflow: "auto", backgroundColor: "#f0f9f0" } }}>
                  {verifyArtifact.content}
                </Code>
                {lastVerifyRunId && (
                  <Anchor
                    component={Link}
                    to={`/projects/${project.id}/tasks/${task.id}/runs/${lastVerifyRunId}`}
                  >
                    View Verify Run Details →
                  </Anchor>
                )}
              </Stack>
            ) : (
              <Text c="dimmed">
                No verification report yet. Click "Verify" to check changes against the plan.
                {!planArtifact && " (No plan exists; verification will do a general diff review.)"}
              </Text>
            )}
          </Card>
        </Tabs.Panel>

        {/* Runs Tab */}
        <Tabs.Panel value="runs" pt="md">
          <Stack>
            {runs.map((r) => (
              <Card
                key={r.id}
                component={Link}
                to={`/projects/${project.id}/tasks/${task.id}/runs/${r.id}`}
                withBorder
                shadow="sm"
                radius="md"
                padding="md"
                style={{ textDecoration: "none", color: "inherit" }}
              >
                <Group justify="space-between" align="center">
                  <div>
                    <Group gap="xs">
                      <Text fw={700}>{r.run_type.toUpperCase()}</Text>
                      <Badge color={r.ended_at ? "green" : "yellow"} variant="light">
                        {r.ended_at ? "Completed" : "Running"}
                      </Badge>
                    </Group>
                    <Text size="sm" c="dimmed">
                      {new Date(r.started_at).toLocaleString()}
                    </Text>
                  </div>
                  <Text size="sm" c="dimmed">
                    {r.model || "—"} • {r.provider || "—"}
                  </Text>
                </Group>
              </Card>
            ))}
            {runs.length === 0 && (
              <Card withBorder padding="md" radius="md" style={{ borderStyle: "dashed" }}>
                <Text c="dimmed" ta="center">
                  No runs yet. Create a Plan or Verify run above.
                </Text>
              </Card>
            )}
          </Stack>
        </Tabs.Panel>
      </Tabs>
    </Box>
  );
}
