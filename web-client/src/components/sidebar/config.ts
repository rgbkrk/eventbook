import { FileText, Bug, ScrollText } from "lucide-react";
import type {
  SidebarSection,
  SidebarItemConfig,
  SidebarConfigOptions,
} from "./types";

// Individual panel configurations
const SIDEBAR_PANELS: Record<
  SidebarSection,
  Omit<SidebarItemConfig, "enabled">
> = {
  audit: {
    id: "audit",
    title: "Audit Log",
    tooltip: "View event history and operations",
    icon: ScrollText,
  },
  metadata: {
    id: "metadata",
    title: "Notebook Info",
    tooltip: "View notebook metadata and statistics",
    icon: FileText,
  },
  debug: {
    id: "debug",
    title: "Debug",
    tooltip: "System health and connection status",
    icon: Bug,
  },
};

// Get all sidebar items based on current options
export function getSidebarItems(
  options: SidebarConfigOptions,
): SidebarItemConfig[] {
  const { isConnected, isDev } = options;

  return [
    {
      ...SIDEBAR_PANELS.audit,
      enabled: true, // Always available
    },
    {
      ...SIDEBAR_PANELS.metadata,
      enabled: true, // Always available
    },
    {
      ...SIDEBAR_PANELS.debug,
      enabled: isDev || !isConnected, // Show in dev mode or when disconnected
    },
  ].filter((item) => item.enabled);
}

// Get configuration for a specific sidebar item
export function getSidebarItemConfig(
  section: SidebarSection,
): SidebarItemConfig {
  const panel = SIDEBAR_PANELS[section];
  if (!panel) {
    throw new Error(`Unknown sidebar section: ${section}`);
  }

  return {
    ...panel,
    enabled: true, // Default to enabled when accessed directly
  };
}

// Get badge count for items that should show counts
export function getSidebarBadgeCount(
  section: SidebarSection,
  options: SidebarConfigOptions,
): number | undefined {
  switch (section) {
    case "audit":
      return options.eventCount > 0 ? options.eventCount : undefined;
    default:
      return undefined;
  }
}
