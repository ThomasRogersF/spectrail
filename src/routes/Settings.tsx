import { useEffect, useState } from "react";
import {
  Card,
  Text,
  Button,
  TextInput,
  PasswordInput,
  NumberInput,
  Group,
  Stack,
  Box,
} from "@mantine/core";
import { PageHeader } from "../ui";
import { getSettings, setSettings } from "../lib/api";

interface SettingsMap {
  provider_name: string;
  base_url: string;
  model: string;
  api_key: string;
  temperature: string;
  max_tokens: string;
}

const DEFAULT_SETTINGS: SettingsMap = {
  provider_name: "openai",
  base_url: "https://api.openai.com/v1",
  model: "gpt-4o",
  api_key: "",
  temperature: "0.2",
  max_tokens: "4000",
};

export default function Settings() {
  const [settings, setSettingsState] = useState<SettingsMap>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saveStatus, setSaveStatus] = useState<"idle" | "success" | "error">("idle");

  useEffect(() => {
    loadSettings();
  }, []);

  async function loadSettings() {
    try {
      const data = await getSettings();
      const map: Record<string, string> = {};
      data.forEach((item) => {
        map[item.key] = item.value;
      });
      setSettingsState({
        provider_name: map.provider_name || DEFAULT_SETTINGS.provider_name,
        base_url: map.base_url || DEFAULT_SETTINGS.base_url,
        model: map.model || DEFAULT_SETTINGS.model,
        api_key: map.api_key || DEFAULT_SETTINGS.api_key,
        temperature: map.temperature || DEFAULT_SETTINGS.temperature,
        max_tokens: map.max_tokens || DEFAULT_SETTINGS.max_tokens,
      });
    } catch (error) {
      console.error("Failed to load settings:", error);
    } finally {
      setLoading(false);
    }
  }

  async function handleSave() {
    setSaving(true);
    setSaveStatus("idle");
    try {
      const pairs = [
        { key: "provider_name", value: settings.provider_name },
        { key: "base_url", value: settings.base_url },
        { key: "model", value: settings.model },
        { key: "api_key", value: settings.api_key },
        { key: "temperature", value: settings.temperature },
        { key: "max_tokens", value: settings.max_tokens },
      ];
      await setSettings(pairs);
      setSaveStatus("success");
    } catch (error) {
      console.error("Failed to save settings:", error);
      setSaveStatus("error");
    } finally {
      setSaving(false);
    }
  }

  return (
    <Box>
      <PageHeader
        title="Settings"
        subtitle="Configure your LLM provider and API credentials"
      />

      <Card withBorder shadow="sm" radius="md" padding="lg">
        <Stack gap="md">
          <Text fw={600} size="lg">LLM Provider Configuration</Text>

          <TextInput
            label="Provider Name"
            placeholder="e.g., openai, anthropic"
            value={settings.provider_name}
            onChange={(e) => setSettingsState({ ...settings, provider_name: e.target.value })}
            disabled={loading || saving}
          />

          <TextInput
            label="Base URL"
            placeholder="https://api.openai.com/v1"
            value={settings.base_url}
            onChange={(e) => setSettingsState({ ...settings, base_url: e.target.value })}
            disabled={loading || saving}
          />

          <TextInput
            label="Model"
            placeholder="e.g., gpt-4o, claude-3-opus"
            value={settings.model}
            onChange={(e) => setSettingsState({ ...settings, model: e.target.value })}
            disabled={loading || saving}
          />

          <PasswordInput
            label="API Key"
            placeholder="Enter your API key"
            value={settings.api_key}
            onChange={(e) => setSettingsState({ ...settings, api_key: e.target.value })}
            disabled={loading || saving}
          />

          <Group grow>
            <NumberInput
              label="Temperature"
              min={0}
              max={2}
              step={0.1}
              value={parseFloat(settings.temperature)}
              onChange={(value) => setSettingsState({ ...settings, temperature: String(value) })}
              disabled={loading || saving}
            />

            <NumberInput
              label="Max Tokens"
              min={1}
              max={128000}
              step={100}
              value={parseInt(settings.max_tokens, 10)}
              onChange={(value) => setSettingsState({ ...settings, max_tokens: String(value) })}
              disabled={loading || saving}
            />
          </Group>

          {saveStatus === "success" && (
            <Text c="green" size="sm">Settings saved successfully!</Text>
          )}

          {saveStatus === "error" && (
            <Text c="red" size="sm">Failed to save settings. Please try again.</Text>
          )}

          <Group justify="flex-end">
            <Button
              onClick={handleSave}
              loading={saving}
              disabled={loading}
            >
              Save Settings
            </Button>
          </Group>
        </Stack>
      </Card>
    </Box>
  );
}
