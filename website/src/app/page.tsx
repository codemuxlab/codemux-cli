"use client";

import { useRef, useState } from "react";
import { AnimatedBeam } from "@/components/magicui/animated-beam";
import { cn } from "@/lib/utils";

// Circle component for nodes
const Circle = ({
  className,
  children,
  ref: forwardedRef,
}: {
  className?: string;
  children: React.ReactNode;
  ref?: React.RefObject<HTMLDivElement>;
}) => {
  return (
    <div
      ref={forwardedRef}
      className={cn(
        "z-10 flex h-12 w-12 items-center justify-center rounded-full bg-white dark:bg-gray-900 border-2 border-gray-200 dark:border-gray-700 text-lg",
        className,
      )}
    >
      {children}
    </div>
  );
};

export default function Home() {
  const [copied, setCopied] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const claude = useRef<HTMLDivElement>(null);
  const gemini = useRef<HTMLDivElement>(null);
  const aider = useRef<HTMLDivElement>(null);
  const codemux = useRef<HTMLDivElement>(null);
  const phone = useRef<HTMLDivElement>(null);
  const tablet = useRef<HTMLDivElement>(null);
  const laptop = useRef<HTMLDivElement>(null);

  const copyToClipboard = async () => {
    try {
      await navigator.clipboard.writeText("npx codemux run claude");
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error("Failed to copy text: ", err);
    }
  };

  return (
    <div className="min-h-screen bg-white dark:bg-black">
      {/* Simple Navigation */}
      <nav className="px-8 py-6 max-w-4xl mx-auto">
        <div className="text-2xl font-medium text-gray-900 dark:text-white">
          CodeMux
        </div>
      </nav>

      {/* Hero */}
      <div className="px-8 py-16 max-w-4xl mx-auto">
        <h1 className="text-5xl font-bold mb-6 text-gray-900 dark:text-white">
          Vibe code from anywhere
        </h1>
        <p className="text-xl text-gray-600 dark:text-gray-300 mb-12 leading-relaxed">
          Terminal multiplexer for AI coding CLIs. Code with Claude, Gemini, and
          Aider from your phone, tablet, or desktop.
        </p>

        {/* Animated Diagram */}
        <div
          className="relative mb-12 h-64 w-full overflow-hidden"
          ref={containerRef}
        >
          {/* AI Agents (Left side) */}
          <div className="absolute left-8 top-8">
            <Circle ref={claude} className="mb-2">
              🤖
            </Circle>
            <div className="text-xs text-center text-gray-500 dark:text-gray-400 w-16">
              Claude
            </div>
          </div>
          <div className="absolute left-8 top-24">
            <Circle ref={gemini} className="mb-2">
              ✨
            </Circle>
            <div className="text-xs text-center text-gray-500 dark:text-gray-400 w-16">
              Gemini
            </div>
          </div>
          <div className="absolute left-8 top-40">
            <Circle ref={aider} className="mb-2">
              🔧
            </Circle>
            <div className="text-xs text-center text-gray-500 dark:text-gray-400 w-16">
              Aider
            </div>
          </div>

          {/* CodeMux Hub (Center) */}
          <div className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2">
            <div
              ref={codemux}
              className="flex h-16 w-16 items-center justify-center rounded-full bg-blue-500 text-white font-bold text-lg"
            >
              CM
            </div>
            <div className="text-xs text-center text-gray-500 dark:text-gray-400 mt-2 w-16">
              CodeMux
            </div>
          </div>

          {/* Devices (Right side) */}
          <div className="absolute right-8 top-8">
            <Circle ref={phone} className="mb-2">
              📱
            </Circle>
            <div className="text-xs text-center text-gray-500 dark:text-gray-400 w-16">
              Phone
            </div>
          </div>
          <div className="absolute right-8 top-24">
            <Circle ref={tablet} className="mb-2">
              📱
            </Circle>
            <div className="text-xs text-center text-gray-500 dark:text-gray-400 w-16">
              Tablet
            </div>
          </div>
          <div className="absolute right-8 top-40">
            <Circle ref={laptop} className="mb-2">
              💻
            </Circle>
            <div className="text-xs text-center text-gray-500 dark:text-gray-400 w-16">
              Desktop
            </div>
          </div>

          {/* Animated Beams - AI Agents to CodeMux */}
          <AnimatedBeam
            containerRef={containerRef}
            fromRef={claude}
            toRef={codemux}
          />
          <AnimatedBeam
            containerRef={containerRef}
            fromRef={gemini}
            toRef={codemux}
            duration={5}
          />
          <AnimatedBeam
            containerRef={containerRef}
            fromRef={aider}
            toRef={codemux}
            duration={6}
          />

          {/* Animated Beams - CodeMux to Devices */}
          <AnimatedBeam
            containerRef={containerRef}
            fromRef={codemux}
            toRef={phone}
            reverse
          />
          <AnimatedBeam
            containerRef={containerRef}
            fromRef={codemux}
            toRef={tablet}
            reverse
            duration={5}
          />
          <AnimatedBeam
            containerRef={containerRef}
            fromRef={codemux}
            toRef={laptop}
            reverse
            duration={6}
          />
        </div>

        <div className="flex items-center gap-4">
          <button
            type="button"
            onClick={copyToClipboard}
            aria-label="Copy command to clipboard"
            className="flex items-center gap-3 px-4 py-2 bg-gray-50 dark:bg-gray-900 border dark:border-gray-800 rounded-lg font-mono text-sm text-gray-900 dark:text-gray-300 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
          >
            <span>npx codemux run claude</span>
            {copied ? (
              <svg
                className="w-4 h-4"
                fill="currentColor"
                viewBox="0 0 20 20"
                role="img"
                aria-label="Copied"
              >
                <path
                  fillRule="evenodd"
                  d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                  clipRule="evenodd"
                />
              </svg>
            ) : (
              <svg
                className="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                role="img"
                aria-label="Copy to clipboard"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                />
              </svg>
            )}
          </button>
          <a
            href="https://github.com/codemuxlab/codemux-cli"
            target="_blank"
            rel="noopener noreferrer"
            className="px-5 py-2 border border-gray-200 dark:border-gray-800 text-gray-900 dark:text-gray-300 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-900 transition-colors"
          >
            GitHub
          </a>
        </div>
      </div>

      {/* Features */}
      <div className="px-8 py-8 max-w-4xl mx-auto">
        <div className="space-y-8">
          <div>
            <h3 className="text-lg font-medium mb-2 text-gray-900 dark:text-white">
              📱 Mobile ready
            </h3>
            <p className="text-gray-600 dark:text-gray-300">
              React Native UI that runs on phones, tablets, and browsers
            </p>
          </div>

          <div>
            <h3 className="text-lg font-medium mb-2 text-gray-900 dark:text-white">
              🤖 AI-first design
            </h3>
            <p className="text-gray-600 dark:text-gray-300">
              Built for "vibe coding" where LLMs drive development instead of
              typing
            </p>
          </div>

          <div>
            <h3 className="text-lg font-medium mb-2 text-gray-900 dark:text-white">
              🔄 Multiple sessions
            </h3>
            <p className="text-gray-600 dark:text-gray-300">
              Run Claude, Gemini, and Aider simultaneously with project
              organization
            </p>
          </div>

          <div>
            <h3 className="text-lg font-medium mb-2 text-gray-900 dark:text-white">
              🛡️ Security focused
            </h3>
            <p className="text-gray-600 dark:text-gray-300">
              Only runs whitelisted AI agents for safe code execution
            </p>
          </div>
        </div>
      </div>

      {/* Installation */}
      <div id="install" className="px-8 py-16 max-w-4xl mx-auto">
        <h2 className="text-2xl font-bold mb-8 text-gray-900 dark:text-white">
          Installation
        </h2>

        <div className="space-y-6">
          <div>
            <div className="text-sm text-gray-500 dark:text-gray-400 mb-2">
              Homebrew (recommended)
            </div>
            <div className="bg-gray-50 dark:bg-gray-900 border dark:border-gray-800 p-4 rounded-lg font-mono text-sm text-gray-900 dark:text-gray-300">
              brew install codemuxlab/tap/codemux
            </div>
          </div>

          <div>
            <div className="text-sm text-gray-500 dark:text-gray-400 mb-2">
              npm
            </div>
            <div className="bg-gray-50 dark:bg-gray-900 border dark:border-gray-800 p-4 rounded-lg font-mono text-sm text-gray-900 dark:text-gray-300">
              npm install -g codemux
            </div>
          </div>

          <div>
            <div className="text-sm text-gray-500 dark:text-gray-400 mb-2">
              Or run directly
            </div>
            <div className="bg-gray-50 dark:bg-gray-900 border dark:border-gray-800 p-4 rounded-lg font-mono text-sm text-gray-900 dark:text-gray-300">
              npx codemux run claude
            </div>
          </div>
        </div>
      </div>

      {/* Usage */}
      <div className="px-8 py-8 max-w-4xl mx-auto">
        <h2 className="text-2xl font-bold mb-8 text-gray-900 dark:text-white">
          Usage
        </h2>

        <div className="space-y-6">
          <div>
            <div className="text-sm text-gray-500 dark:text-gray-400 mb-2">
              Quick start
            </div>
            <div className="bg-gray-50 dark:bg-gray-900 border dark:border-gray-800 p-4 rounded-lg font-mono text-sm text-gray-900 dark:text-gray-300">
              codemux run claude --open
            </div>
          </div>

          <div>
            <div className="text-sm text-gray-500 dark:text-gray-400 mb-2">
              Continue previous session
            </div>
            <div className="bg-gray-50 dark:bg-gray-900 border dark:border-gray-800 p-4 rounded-lg font-mono text-sm text-gray-900 dark:text-gray-300">
              codemux run claude --continue
            </div>
          </div>

          <div>
            <div className="text-sm text-gray-500 dark:text-gray-400 mb-2">
              Daemon mode
            </div>
            <div className="bg-gray-50 dark:bg-gray-900 border dark:border-gray-800 p-4 rounded-lg font-mono text-sm space-y-2 text-gray-900 dark:text-gray-300">
              <div>codemux daemon</div>
              <div>codemux add-project ~/my-project</div>
            </div>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="px-8 py-12 max-w-4xl mx-auto border-t border-gray-100 dark:border-gray-800 mt-16">
        <div className="flex justify-between items-center text-sm text-gray-500 dark:text-gray-400">
          <div>© 2024 CodeMux Lab</div>
          <div className="flex gap-6">
            <a
              href="https://github.com/codemuxlab/codemux-cli"
              className="hover:text-gray-700 dark:hover:text-gray-300"
            >
              GitHub
            </a>
            <a
              href="https://github.com/codemuxlab/codemux-cli/issues"
              className="hover:text-gray-700 dark:hover:text-gray-300"
            >
              Issues
            </a>
          </div>
        </div>
      </div>
    </div>
  );
}
