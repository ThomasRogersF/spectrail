import { Group, Title, Text } from '@mantine/core';
import type { ReactNode } from 'react';

interface PageHeaderProps {
  title: string;
  subtitle?: ReactNode;
  rightActions?: ReactNode;
}

export function PageHeader({ title, subtitle, rightActions }: PageHeaderProps) {
  return (
    <Group justify="space-between" align="flex-start" mb="xl">
      <div>
        <Title order={2}>{title}</Title>
        {subtitle && (
          <Text c="dimmed" size="sm">
            {subtitle}
          </Text>
        )}
      </div>
      {rightActions}
    </Group>
  );
}
