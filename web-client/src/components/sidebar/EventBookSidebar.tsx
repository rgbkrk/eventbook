import React, { useState } from "react";
import { Button } from "@/components/ui/button";
import { Link } from "react-router-dom";
import type { EventBookSidebarProps, SidebarSection } from "./types";
import { getSidebarItems, getSidebarItemConfig } from "./config";
import { AuditLogPanel, MetadataPanel, DebugPanel } from "./panels";
import type { SidebarPanelProps } from "./types";
import { X, ArrowLeft } from "lucide-react";

// Panel component mapping
const PANEL_COMPONENTS: Record<SidebarSection, React.FC<SidebarPanelProps>> = {
  audit: AuditLogPanel,
  metadata: MetadataPanel,
  debug: DebugPanel,
};

// Simple EventBook logo component
const EventBookLogo: React.FC = () => (
  <div className="flex items-center justify-center h-8 w-8 rounded bg-primary text-primary-foreground text-sm font-bold">
    EB
  </div>
);

export const EventBookSidebar: React.FC<EventBookSidebarProps> = ({
  notebookId,
  notebookState,
  onUpdate,
  className = "",
}) => {
  const [activeSection, setActiveSection] = useState<SidebarSection | null>(
    null,
  );

  const toggleSection = (section: SidebarSection) => {
    setActiveSection(activeSection === section ? null : section);
  };

  const sidebarItems = getSidebarItems({
    isConnected: true, // TODO: Get from connection state
    isDev: import.meta.env.DEV,
    activeSection,
    eventCount: 0, // TODO: Pass actual event count
  });

  const renderPanelContent = () => {
    if (!activeSection) return null;

    const PanelComponent = PANEL_COMPONENTS[activeSection];
    const panelProps: SidebarPanelProps = {
      notebookId,
      notebookState,
      onUpdate,
    };

    return <PanelComponent {...panelProps} />;
  };

  const activeItem = activeSection ? getSidebarItemConfig(activeSection) : null;

  return (
    <>
      {/* Desktop: Icon-only sidebar (hidden on mobile) */}
      <div
        className={`fixed top-0 left-0 z-40 hidden h-full w-12 flex-col items-center border-r bg-gray-50 py-4 lg:flex ${className}`}
      >
        {/* Logo and back navigation */}
        <div className="mb-4 flex flex-col items-center space-y-2">
          <Link
            to="/"
            className="group/logo relative flex h-8 w-8 items-center justify-center rounded hover:bg-gray-200"
            title="Back to Home"
          >
            <span className="relative transition-opacity group-hover/logo:opacity-20">
              <EventBookLogo />
            </span>
            <ArrowLeft className="absolute h-4 w-4 opacity-0 transition-opacity group-hover/logo:opacity-100" />
          </Link>
        </div>

        {/* Sidebar items */}
        <div className="flex flex-col space-y-2">
          {sidebarItems.map((item) => {
            const Icon = item.icon;
            const isActive = activeSection === item.id;

            return (
              <Button
                key={item.id}
                variant="ghost"
                size="icon"
                onClick={() => toggleSection(item.id)}
                className={`h-8 w-8 ${
                  isActive
                    ? "bg-blue-100 text-blue-600 hover:bg-blue-200"
                    : "text-gray-600 hover:bg-gray-200 hover:text-gray-900"
                }`}
                title={item.tooltip}
              >
                <Icon className="h-4 w-4" />
              </Button>
            );
          })}
        </div>
      </div>

      {/* Mobile: Bottom navigation bar (hidden on desktop) */}
      <div className="fixed right-0 bottom-0 left-0 z-40 flex items-center justify-center border-t bg-white p-2 shadow-lg lg:hidden">
        {/* Back button */}
        <div className="flex w-full items-center justify-between px-2">
          <Link
            to="/"
            className="flex h-10 w-10 items-center justify-center rounded-lg bg-gray-100 hover:bg-gray-200"
            title="Back to Home"
          >
            <ArrowLeft className="h-5 w-5 text-gray-600" />
          </Link>

          {/* Mobile sidebar items */}
          <div className="flex items-center space-x-1">
            {sidebarItems.map((item) => {
              const Icon = item.icon;
              const isActive = activeSection === item.id;

              return (
                <Button
                  key={item.id}
                  variant="ghost"
                  size="icon"
                  onClick={() => toggleSection(item.id)}
                  className={`h-10 w-10 ${
                    isActive
                      ? "bg-blue-100 text-blue-600 hover:bg-blue-200"
                      : "text-gray-600 hover:bg-gray-200 hover:text-gray-900"
                  }`}
                  title={item.tooltip}
                >
                  <Icon className="h-5 w-5" />
                </Button>
              );
            })}
          </div>

          {/* Spacer for visual balance */}
          <div className="h-10 w-10" />
        </div>
      </div>

      {/* Desktop: Slide-out panel from left */}
      {activeSection && (
        <>
          {/* Desktop backdrop */}
          <div
            className="fixed inset-0 z-30 hidden bg-black/20 lg:block"
            onClick={() => setActiveSection(null)}
          />

          {/* Desktop panel */}
          <div className="fixed top-0 left-12 z-50 hidden h-full w-96 overflow-auto border-r bg-white shadow-lg lg:block">
            <div className="flex items-center justify-between border-b px-4 py-3">
              <h3 className="font-medium text-gray-900">{activeItem?.title}</h3>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setActiveSection(null)}
                className="h-8 w-8 p-0"
              >
                <X className="h-4 w-4" />
              </Button>
            </div>

            <div className="p-4">{renderPanelContent()}</div>
          </div>
        </>
      )}

      {/* Mobile: Bottom sheet panel */}
      {activeSection && (
        <>
          {/* Mobile backdrop */}
          <div
            className="fixed inset-0 z-30 bg-black/20 lg:hidden"
            onClick={() => setActiveSection(null)}
          />

          {/* Mobile bottom sheet */}
          <div className="fixed right-0 bottom-0 left-0 z-50 max-h-[70vh] overflow-auto rounded-t-xl border-t bg-white shadow-2xl lg:hidden">
            {/* Handle bar */}
            <div className="flex justify-center p-2">
              <div className="h-1 w-12 rounded-full bg-gray-300" />
            </div>

            {/* Header */}
            <div className="flex items-center justify-between border-b px-4 py-3">
              <h3 className="text-lg font-medium text-gray-900">
                {activeItem?.title}
              </h3>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setActiveSection(null)}
                className="h-8 w-8 p-0"
              >
                <X className="h-4 w-4" />
              </Button>
            </div>

            {/* Content */}
            <div className="p-4 pb-20">{renderPanelContent()}</div>
          </div>
        </>
      )}
    </>
  );
};
