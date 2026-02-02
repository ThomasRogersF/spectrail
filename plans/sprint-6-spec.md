# Sprint 6: UI/UX Upgrade with Mantine

## Overview
Replace inline styles with Mantine components for a polished, consistent desktop app UI.

## Constraints
- UI-only: No backend changes
- Keep existing routes and API calls
- Maintain all existing functionality
- TypeScript build must pass

---

## A) Install Mantine Dependencies

```bash
pnpm add @mantine/core @mantine/hooks @mantine/notifications @tabler/icons-react
```

---

## B) App Setup

### Update `src/main.tsx`

```typescript
import React from 'react';
import ReactDOM from 'react-dom/client';
import { MantineProvider } from '@mantine/core';
import { Notifications } from '@mantine/notifications';
import { BrowserRouter } from 'react-router-dom';
import App from './App';
import '@mantine/core/styles.css';
import '@mantine/notifications/styles.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <BrowserRouter>
      <MantineProvider defaultColorScheme="light">
        <Notifications />
        <App />
      </MantineProvider>
    </BrowserRouter>
  </React.StrictMode>
);
```

---

## C) Create UI Components

### `src/ui/AppLayout.tsx`

```typescript
import { AppShell, Burger, Group, NavLink, Text, ActionIcon, useMantineColorScheme } from '@mantine/core';
import { useDisclosure } from '@mantine/hooks';
import { IconFolder, IconSettings, IconSun, IconMoon } from '@tabler/icons-react';
import { Link, useLocation, Outlet } from 'react-router-dom';

export function AppLayout() {
  const [opened, { toggle }] = useDisclosure();
  const { colorScheme, toggleColorScheme } = useMantineColorScheme();
  const location = useLocation();

  return (
    <AppShell
      header={{ height: 60 }}
      navbar={{ width: 200, breakpoint: 'sm', collapsed: { mobile: !opened } }}
      padding="md"
    >
      <AppShell.Header>
        <Group h="100%" px="md" justify="space-between">
          <Group>
            <Burger opened={opened} onClick={toggle} hiddenFrom="sm" size="sm" />
            <Text fw={700} size="lg">SpecTrail</Text>
          </Group>
          <Group>
            <Text size="xs" c="dimmed" visibleFrom="sm">Plan → Phase → Verify → Replay</Text>
            <ActionIcon variant="default" onClick={toggleColorScheme}>
              {colorScheme === 'dark' ? <IconSun size={18} /> : <IconMoon size={18} />}
            </ActionIcon>
          </Group>
        </Group>
      </AppShell.Header>

      <AppShell.Navbar p="md">
        <NavLink
          component={Link}
          to="/projects"
          label="Projects"
          leftSection={<IconFolder size={18} />}
          active={location.pathname.startsWith('/projects')}
        />
        <NavLink
          component={Link}
          to="/settings"
          label="Settings"
          leftSection={<IconSettings size={18} />}
          active={location.pathname === '/settings'}
        />
      </AppShell.Navbar>

      <AppShell.Main>
        <Outlet />
      </AppShell.Main>
    </AppShell>
  );
}
```

### `src/ui/PageHeader.tsx`

```typescript
import { Group, Title, Text, Button } from '@mantine/core';

interface PageHeaderProps {
  title: string;
  subtitle?: string;
  rightActions?: React.ReactNode;
}

export function PageHeader({ title, subtitle, rightActions }: PageHeaderProps) {
  return (
    <Group justify="space-between" align="flex-start" mb="xl">
      <div>
        <Title order={2}>{title}</Title>
        {subtitle && <Text c="dimmed" size="sm">{subtitle}</Text>}
      </div>
      {rightActions}
    </Group>
  );
}
```

### `src/ui/SectionCard.tsx`

```typescript
import { Card, Title, Group } from '@mantine/core';

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
```

### `src/ui/CopyAction.tsx`

```typescript
import { ActionIcon, Tooltip } from '@mantine/core';
import { IconCopy } from '@tabler/icons-react';
import { notifications } from '@mantine/notifications';

interface CopyActionProps {
  text: string;
  label?: string;
}

export function CopyAction({ text, label = 'Copy' }: CopyActionProps) {
  const handleCopy = async () => {
    await navigator.clipboard.writeText(text);
    notifications.show({
      title: 'Copied!',
      message: 'Content copied to clipboard',
      color: 'green',
    });
  };

  return (
    <Tooltip label={label}>
      <ActionIcon variant="light" onClick={handleCopy}>
        <IconCopy size={16} />
      </ActionIcon>
    </Tooltip>
  );
}
```

---

## D) Refactor Routes

### `src/routes/Projects.tsx`

Use: PageHeader, SimpleGrid/Table, Card, Button, TextInput, Badge

### `src/routes/ProjectDetail.tsx`

Use: PageHeader, Card, Table, Badge, Button, TextInput, SegmentedControl

### `src/routes/TaskDetail.tsx`

Convert to Tabs layout:
- Tabs with 3 panels: Plan, Verify, Runs
- Use LoadingOverlay during operations
- Alert for errors

### `src/routes/RunDetail.tsx`

Use Tabs: Messages | Tool Calls
- Messages: Timeline or stacked Cards with role badges
- Tool Calls: Accordion with JSON content

### `src/routes/Settings.tsx`

Use: Card, TextInput, NumberInput, Checkbox, Button, Divider
Group settings logically with Stack and Group.

---

## E) App.tsx Update

Wrap routes with AppLayout and organize route structure.

---

## Acceptance Criteria
- [ ] Mantine installed and providers wired
- [ ] AppLayout with navbar + header + color scheme toggle
- [ ] All 5 routes refactored to Mantine components
- [ ] No inline styles remaining (or minimal)
- [ ] Dark mode toggle works
- [ ] Copy notifications work
- [ ] TypeScript build passes
- [ ] All existing functionality preserved
