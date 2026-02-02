import { AppShell, Burger, Group, NavLink, Text, ActionIcon, useMantineColorScheme } from '@mantine/core';
import { useDisclosure } from '@mantine/hooks';
import { IconFolder, IconSettings, IconSun, IconMoon } from '@tabler/icons-react';
import { Link, useLocation, Outlet } from 'react-router-dom';
import { useEffect } from 'react';
import { getSetting, setSetting } from '../lib/api';

export function AppLayout() {
  const [opened, { toggle }] = useDisclosure();
  const location = useLocation();
  const { colorScheme, toggleColorScheme } = useMantineColorScheme();

  // Load color scheme from settings on mount
  useEffect(() => {
    getSetting('ui_color_scheme').then((scheme) => {
      if (scheme === 'light' || scheme === 'dark') {
        // Apply the stored color scheme if different from current
        if (scheme !== colorScheme) {
          toggleColorScheme();
        }
      }
    });
  }, []);

  const handleToggleColorScheme = async () => {
    const newScheme = colorScheme === 'light' ? 'dark' : 'light';
    toggleColorScheme();
    await setSetting('ui_color_scheme', newScheme);
  };

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
            <ActionIcon variant="default" onClick={handleToggleColorScheme}>
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
