import { Card, Title, Group } from "@mantine/core";

interface SectionCardProps {
  title?: string;
  rightActions?: React.ReactNode;
  children: React.ReactNode;
}

export function SectionCard({ title, rightActions, children }: SectionCardProps) {
  return (
    <Card withBorder shadow="sm" radius="md" mb="lg">
      {(title || rightActions) && (
        <Group justify="space-between" mb="md">
          {title && <Title order={4}>{title}</Title>}
          {rightActions}
        </Group>
      )}
      {children}
    </Card>
  );
}
