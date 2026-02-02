import { ActionIcon, Tooltip } from "@mantine/core";
import { IconCopy } from "@tabler/icons-react";
import { notifications } from "@mantine/notifications";

interface CopyActionProps {
  text: string;
  label?: string;
}

export function CopyAction({ text, label = "Copy" }: CopyActionProps) {
  const handleCopy = async () => {
    await navigator.clipboard.writeText(text);
    notifications.show({
      title: "Copied!",
      message: "Content copied to clipboard",
      color: "green",
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
