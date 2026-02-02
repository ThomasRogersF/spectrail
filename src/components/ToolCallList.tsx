import React from "react";
import {
  Accordion,
  Code,
  Text,
  Stack,
  Badge,
  Card,
  Group,
} from "@mantine/core";
import { IconTool } from "@tabler/icons-react";

interface ToolCall {
  id: string;
  name: string;
  args_json: string;
  result_json: string;
  created_at: string;
}

interface ToolCallListProps {
  toolCalls: ToolCall[];
}

export default function ToolCallList({ toolCalls }: ToolCallListProps) {
  if (toolCalls.length === 0) {
    return (
      <Card withBorder padding="md" radius="md" style={{ borderStyle: "dashed" }}>
        <Text c="dimmed" ta="center">No tool calls</Text>
      </Card>
    );
  }

  return (
    <Accordion variant="separated" chevronPosition="right">
      {toolCalls.map((tc) => {
        let args: unknown;
        let result: unknown;

        try {
          args = JSON.parse(tc.args_json);
        } catch {
          args = tc.args_json;
        }

        try {
          result = JSON.parse(tc.result_json);
        } catch {
          result = tc.result_json;
        }

        return (
          <Accordion.Item key={tc.id} value={tc.id}>
            <Accordion.Control icon={<IconTool size={16} />}>
              <Group gap="xs">
                <Text fw={600}>{tc.name}</Text>
                <Badge size="xs" variant="light">
                  {new Date(tc.created_at).toLocaleTimeString()}
                </Badge>
              </Group>
            </Accordion.Control>
            <Accordion.Panel>
              <Stack>
                <div>
                  <Text size="xs" c="dimmed" tt="uppercase" mb="xs">
                    Arguments
                  </Text>
                  <Code block styles={{ root: { maxHeight: 300, overflow: "auto" } }}>
                    {JSON.stringify(args, null, 2)}
                  </Code>
                </div>

                <div>
                  <Text size="xs" c="dimmed" tt="uppercase" mb="xs">
                    Result
                  </Text>
                  <Code block styles={{ root: { maxHeight: 300, overflow: "auto" } }}>
                    {JSON.stringify(result, null, 2)}
                  </Code>
                </div>
              </Stack>
            </Accordion.Panel>
          </Accordion.Item>
        );
      })}
    </Accordion>
  );
}
