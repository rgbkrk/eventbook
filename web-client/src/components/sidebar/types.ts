import type { NotebookState } from "@/types/eventbook";
import { LucideIcon } from "lucide-react";

// Available sidebar sections/panels
export type SidebarSection = "audit" | "metadata" | "debug";

// Props passed to each panel component
export interface SidebarPanelProps {
  notebookId: string;
  notebookState: NotebookState;
  onUpdate: () => void;
}

// Configuration for each sidebar item
export interface SidebarItemConfig {
  id: SidebarSection;
  title: string;
  tooltip: string;
  icon: LucideIcon;
  enabled: boolean;
}

// Props for the main sidebar component
export interface EventBookSidebarProps {
  notebookId: string;
  notebookState: NotebookState;
  onUpdate: () => void;
  className?: string;
}

// Options for sidebar configuration
export interface SidebarConfigOptions {
  isConnected: boolean;
  isDev: boolean;
  activeSection: SidebarSection | null;
  eventCount: number;
}
