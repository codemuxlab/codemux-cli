'use client';

import { useState } from 'react';
import { Copy, Check, Monitor, Apple } from 'lucide-react';

type Platform = 'unix' | 'windows';

interface InstallCommand {
  platform: Platform;
  title: string;
  command: string;
  icon: React.ReactNode;
}

const installCommands: InstallCommand[] = [
  {
    platform: 'unix',
    title: 'macOS / Linux',
    command: 'curl -sSf https://codemux.dev/install.sh | sh',
    icon: <Apple className="h-4 w-4" />,
  },
  {
    platform: 'windows',
    title: 'Windows',
    command: 'irm https://codemux.dev/install.ps1 | iex',
    icon: <Monitor className="h-4 w-4" />,
  },
];

export function InstallationTabs() {
  const [activeTab, setActiveTab] = useState<Platform>('unix');
  const [copiedStates, setCopiedStates] = useState<Record<Platform, boolean>>({
    unix: false,
    windows: false,
  });

  const copyToClipboard = async (text: string, platform: Platform) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedStates(prev => ({ ...prev, [platform]: true }));
      
      // Reset copied state after 2 seconds
      setTimeout(() => {
        setCopiedStates(prev => ({ ...prev, [platform]: false }));
      }, 2000);
    } catch (err) {
      console.error('Failed to copy text: ', err);
    }
  };

  return (
    <div className="rounded-lg border border-fd-border bg-fd-card p-6">
      <h3 className="mb-4 text-lg font-semibold">Install CodeMux</h3>
      
      {/* Tab Navigation */}
      <div className="mb-4 flex space-x-1 rounded-lg bg-fd-muted p-1">
        {installCommands.map((cmd) => (
          <button
            key={cmd.platform}
            onClick={() => setActiveTab(cmd.platform)}
            className={`flex items-center space-x-2 rounded-md px-3 py-2 text-sm font-medium transition-colors ${
              activeTab === cmd.platform
                ? 'bg-fd-background text-fd-foreground shadow-sm'
                : 'text-fd-muted-foreground hover:text-fd-foreground'
            }`}
          >
            {cmd.icon}
            <span>{cmd.title}</span>
          </button>
        ))}
      </div>

      {/* Tab Content */}
      {installCommands.map((cmd) => (
        <div
          key={cmd.platform}
          className={activeTab === cmd.platform ? 'block' : 'hidden'}
        >
          <div className="relative">
            <pre className="overflow-x-auto rounded-md bg-fd-muted p-4 pr-12">
              <code className="text-sm">{cmd.command}</code>
            </pre>
            <button
              onClick={() => copyToClipboard(cmd.command, cmd.platform)}
              className="absolute right-2 top-2 rounded-md p-2 text-fd-muted-foreground transition-colors hover:bg-fd-background hover:text-fd-foreground"
              title="Copy to clipboard"
            >
              {copiedStates[cmd.platform] ? (
                <Check className="h-4 w-4 text-green-500" />
              ) : (
                <Copy className="h-4 w-4" />
              )}
            </button>
          </div>
        </div>
      ))}

      {/* Alternative Methods */}
      <div className="mt-6">
        <h4 className="mb-3 text-base font-medium text-fd-muted-foreground">
          Alternative Methods
        </h4>
        <div className="space-y-2 text-sm">
          <div className="flex items-center justify-between rounded-md bg-fd-muted/50 px-3 py-2">
            <span>Homebrew (macOS/Linux)</span>
            <code className="text-xs">brew install codemuxlab/tap/codemux</code>
          </div>
          <div className="flex items-center justify-between rounded-md bg-fd-muted/50 px-3 py-2">
            <span>npm (Node.js)</span>
            <code className="text-xs">npm install codemux@latest</code>
          </div>
        </div>
      </div>
    </div>
  );
}