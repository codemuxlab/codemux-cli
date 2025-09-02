'use client';

import { useState, useEffect } from 'react';
import { ChevronRight, Copy, Check, Download } from 'lucide-react';

type Platform = 'macos' | 'linux' | 'windows' | 'unknown';

function detectPlatform(): Platform {
  if (typeof window === 'undefined') return 'unknown';
  
  const userAgent = window.navigator.userAgent.toLowerCase();
  
  if (userAgent.includes('mac')) return 'macos';
  if (userAgent.includes('linux')) return 'linux';
  if (userAgent.includes('windows')) return 'windows';
  
  return 'unknown';
}

export function SmartInstallButton() {
  const [platform, setPlatform] = useState<Platform>('unknown');
  const [copied, setCopied] = useState(false);
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
    setPlatform(detectPlatform());
  }, []);

  const installCommand = 'curl -sSfL https://codemux.dev/install.sh | sh';
  
  const handleClick = async () => {
    if (platform === 'macos' || platform === 'linux') {
      // Copy command for Unix platforms
      try {
        await navigator.clipboard.writeText(installCommand);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      } catch (err) {
        console.error('Failed to copy:', err);
        // Fallback to scrolling if clipboard fails
        scrollToQuickStart();
      }
    } else {
      // Scroll to installation section for Windows/unknown
      scrollToQuickStart();
    }
  };

  const scrollToQuickStart = () => {
    document.getElementById('quick-start')?.scrollIntoView({ 
      behavior: 'smooth',
      block: 'start'
    });
  };

  // Don't render until mounted to avoid hydration issues
  if (!mounted) {
    return (
      <button className="inline-flex items-center justify-center rounded-lg bg-fd-primary px-6 py-3 text-sm font-medium text-fd-primary-foreground transition-colors hover:bg-fd-primary/90">
        Get Started
        <ChevronRight className="ml-2 h-4 w-4" />
      </button>
    );
  }

  const isUnixPlatform = platform === 'macos' || platform === 'linux';
  
  return (
    <div className="flex flex-col gap-3">
      {isUnixPlatform ? (
        <>
          <div className="flex items-center gap-2 rounded-lg border border-fd-border bg-fd-secondary p-3">
            <code className="flex-1 text-sm text-fd-foreground">{installCommand}</code>
            <button
              onClick={handleClick}
              className="rounded-md p-2 text-fd-muted-foreground transition-colors hover:bg-fd-background hover:text-fd-foreground"
              title="Copy to clipboard"
            >
              {copied ? (
                <Check className="h-4 w-4 text-green-500" />
              ) : (
                <Copy className="h-4 w-4" />
              )}
            </button>
          </div>
          
          {!copied && (
            <p className="text-xs text-fd-muted-foreground text-center">
              For {platform === 'macos' ? 'macOS' : 'Linux'} • 
              <button 
                onClick={scrollToQuickStart}
                className="ml-1 underline hover:no-underline"
              >
                See all options
              </button>
            </p>
          )}
          
          {copied && (
            <p className="text-xs text-green-600 text-center">
              Now paste and run in your terminal
            </p>
          )}
        </>
      ) : (
        <button
          onClick={handleClick}
          className="inline-flex items-center justify-center rounded-lg bg-fd-primary px-6 py-3 text-sm font-medium text-fd-primary-foreground transition-colors hover:bg-fd-primary/90"
        >
          <Download className="mr-2 h-4 w-4" />
          Get Started
          <ChevronRight className="ml-2 h-4 w-4" />
        </button>
      )}
    </div>
  );
}