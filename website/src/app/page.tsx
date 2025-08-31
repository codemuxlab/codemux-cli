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
  ref?: React.RefObject<HTMLDivElement | null>;
}) => {
  return (
    <div
      ref={forwardedRef}
      className={cn(
        "z-10 flex h-12 w-12 items-center justify-center rounded-full bg-white/10 backdrop-blur border border-white/20 text-lg text-white",
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
      await navigator.clipboard.writeText("npx codemux claude");
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error("Failed to copy text: ", err);
    }
  };

  return (
    <div className="min-h-screen bg-[#0f0f0f] relative overflow-hidden">
      {/* Background gradient */}
      <div className="absolute inset-0">
        <div className="absolute inset-0 bg-gradient-to-br from-blue-500/10 via-transparent to-purple-500/10" />
        <div className="absolute top-0 left-1/2 -translate-x-1/2 w-full h-full bg-gradient-radial from-white/5 via-transparent to-transparent" />
      </div>
      
      {/* Navigation */}
      <nav className="relative z-10 px-8 py-6">
        <div className="max-w-6xl mx-auto flex justify-center">
          <div className="text-2xl font-bold font-jakarta bg-gradient-to-r from-white to-white/80 bg-clip-text text-transparent">
            CodeMux
          </div>
        </div>
      </nav>

      {/* Hero */}
      <div className="relative z-10 px-8 py-20 max-w-6xl mx-auto text-center">
        <h1 className="text-7xl font-bold mb-8 font-jakarta">
          <span className="bg-gradient-to-r from-white via-white to-white/70 bg-clip-text text-transparent">
            Vibe code from
          </span>
          <br />
          <span className="bg-gradient-to-r from-blue-400 via-purple-400 to-cyan-400 bg-clip-text text-transparent">
            anywhere
          </span>
        </h1>
        <p className="text-xl text-white/70 mb-16 leading-relaxed max-w-3xl mx-auto">
          Terminal multiplexer for AI coding CLIs. Code with Claude, Gemini, and
          Aider from your phone, tablet, or desktop.
        </p>

        {/* Animated Diagram */}
        <div
          className="relative mb-16 h-80 w-full overflow-hidden rounded-2xl bg-gradient-to-br from-white/5 to-white/0 backdrop-blur border border-white/10"
          ref={containerRef}
        >
          {/* AI Agents (Left side) */}
          <div className="absolute left-12 top-12">
            <Circle ref={claude} className="mb-3">
              🤖
            </Circle>
            <div className="text-sm text-center text-white/70 w-16 font-medium">
              Claude
            </div>
          </div>
          <div className="absolute left-12 top-32">
            <Circle ref={gemini} className="mb-3">
              ✨
            </Circle>
            <div className="text-sm text-center text-white/70 w-16 font-medium">
              Gemini
            </div>
          </div>
          <div className="absolute left-12 top-52">
            <Circle ref={aider} className="mb-3">
              🔧
            </Circle>
            <div className="text-sm text-center text-white/70 w-16 font-medium">
              Aider
            </div>
          </div>

          {/* CodeMux Hub (Center) */}
          <div className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2">
            <div
              ref={codemux}
              className="flex h-20 w-20 items-center justify-center rounded-full bg-gradient-to-br from-blue-500 to-purple-600 text-white font-bold text-xl shadow-2xl shadow-blue-500/25"
            >
              CM
            </div>
            <div className="text-sm text-center text-white font-medium mt-3 w-20">
              CodeMux
            </div>
          </div>

          {/* Devices (Right side) */}
          <div className="absolute right-12 top-12">
            <Circle ref={phone} className="mb-3">
              📱
            </Circle>
            <div className="text-sm text-center text-white/70 w-16 font-medium">
              Phone
            </div>
          </div>
          <div className="absolute right-12 top-32">
            <Circle ref={tablet} className="mb-3">
              📲
            </Circle>
            <div className="text-sm text-center text-white/70 w-16 font-medium">
              Tablet
            </div>
          </div>
          <div className="absolute right-12 top-52">
            <Circle ref={laptop} className="mb-3">
              💻
            </Circle>
            <div className="text-sm text-center text-white/70 w-16 font-medium">
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

        <div className="flex items-center justify-center gap-6">
          <button
            type="button"
            onClick={copyToClipboard}
            aria-label="Copy command to clipboard"
            className="group flex items-center gap-3 px-6 py-3 bg-white/10 backdrop-blur border border-white/20 rounded-xl font-mono text-sm text-white cursor-pointer hover:bg-white/20 hover:border-white/30 transition-all duration-300 hover:scale-105 hover:shadow-2xl hover:shadow-blue-500/25"
          >
            <span>npx codemux claude</span>
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
            className="group px-6 py-3 border border-white/20 text-white rounded-xl hover:bg-white/10 hover:border-white/30 transition-all duration-300 hover:scale-105 font-medium backdrop-blur"
          >
            <span className="group-hover:text-blue-300 transition-colors">GitHub</span>
          </a>
        </div>
      </div>

      {/* Features */}
      <div className="relative z-10 px-8 py-20 max-w-6xl mx-auto">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
          <div className="group p-8 rounded-2xl bg-gradient-to-br from-white/5 to-white/0 backdrop-blur border border-white/10 hover:border-white/20 transition-all duration-300 hover:scale-[1.02]">
            <div className="text-3xl mb-4">📱</div>
            <h3 className="text-xl font-bold mb-3 text-white font-jakarta">
              Mobile ready
            </h3>
            <p className="text-white/70 leading-relaxed">
              React Native UI that runs on phones, tablets, and browsers. Code from anywhere with touch-optimized interface.
            </p>
          </div>

          <div className="group p-8 rounded-2xl bg-gradient-to-br from-white/5 to-white/0 backdrop-blur border border-white/10 hover:border-white/20 transition-all duration-300 hover:scale-[1.02]">
            <div className="text-3xl mb-4">🤖</div>
            <h3 className="text-xl font-bold mb-3 text-white font-jakarta">
              AI-first design
            </h3>
            <p className="text-white/70 leading-relaxed">
              Built for "vibe coding" where LLMs drive development instead of typing. Enhanced terminal for AI workflows.
            </p>
          </div>

          <div className="group p-8 rounded-2xl bg-gradient-to-br from-white/5 to-white/0 backdrop-blur border border-white/10 hover:border-white/20 transition-all duration-300 hover:scale-[1.02]">
            <div className="text-3xl mb-4">🔄</div>
            <h3 className="text-xl font-bold mb-3 text-white font-jakarta">
              Multiple sessions
            </h3>
            <p className="text-white/70 leading-relaxed">
              Run Claude, Gemini, and Aider simultaneously with project organization and session management.
            </p>
          </div>

          <div className="group p-8 rounded-2xl bg-gradient-to-br from-white/5 to-white/0 backdrop-blur border border-white/10 hover:border-white/20 transition-all duration-300 hover:scale-[1.02]">
            <div className="text-3xl mb-4">🛡️</div>
            <h3 className="text-xl font-bold mb-3 text-white font-jakarta">
              Security focused
            </h3>
            <p className="text-white/70 leading-relaxed">
              Only runs whitelisted AI agents for safe code execution. Secure sandbox for AI-driven development.
            </p>
          </div>
        </div>
      </div>

      {/* Installation */}
      <div id="install" className="relative z-10 px-8 py-20 max-w-6xl mx-auto">
        <h2 className="text-4xl font-bold mb-12 text-white text-center font-jakarta">
          Get Started
        </h2>

        <div className="space-y-8">
          <div className="group">
            <div className="text-lg text-white/80 mb-4 font-medium">
              Homebrew (recommended)
            </div>
            <div className="bg-black/40 backdrop-blur border border-white/20 p-6 rounded-2xl font-mono text-lg text-white group-hover:border-white/30 transition-all duration-300">
              brew install codemuxlab/tap/codemux
            </div>
          </div>

          <div className="group">
            <div className="text-lg text-white/80 mb-4 font-medium">
              npm
            </div>
            <div className="bg-black/40 backdrop-blur border border-white/20 p-6 rounded-2xl font-mono text-lg text-white group-hover:border-white/30 transition-all duration-300">
              npm install -g codemux
            </div>
          </div>

          <div className="group">
            <div className="text-lg text-white/80 mb-4 font-medium">
              Or run directly
            </div>
            <div className="bg-black/40 backdrop-blur border border-white/20 p-6 rounded-2xl font-mono text-lg text-white group-hover:border-white/30 transition-all duration-300">
              npx codemux claude
            </div>
          </div>
        </div>
      </div>

      {/* Usage */}
      <div className="relative z-10 px-8 py-20 max-w-6xl mx-auto">
        <h2 className="text-4xl font-bold mb-12 text-white text-center font-jakarta">
          Usage Examples
        </h2>

        <div className="space-y-8">
          <div className="group">
            <div className="text-lg text-white/80 mb-4 font-medium">
              Quick start with web UI
            </div>
            <div className="bg-black/40 backdrop-blur border border-white/20 p-6 rounded-2xl font-mono text-lg text-white group-hover:border-white/30 transition-all duration-300">
              codemux claude --open
            </div>
          </div>

          <div className="group">
            <div className="text-lg text-white/80 mb-4 font-medium">
              Continue previous session
            </div>
            <div className="bg-black/40 backdrop-blur border border-white/20 p-6 rounded-2xl font-mono text-lg text-white group-hover:border-white/30 transition-all duration-300">
              codemux claude --continue
            </div>
          </div>

          <div className="group">
            <div className="text-lg text-white/80 mb-4 font-medium">
              Server mode for multiple projects
            </div>
            <div className="bg-black/40 backdrop-blur border border-white/20 p-6 rounded-2xl font-mono text-lg space-y-3 text-white group-hover:border-white/30 transition-all duration-300">
              <div>codemux server start</div>
              <div>codemux add-project ~/my-project</div>
            </div>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="relative z-10 px-8 py-16 max-w-6xl mx-auto border-t border-white/10 mt-20">
        <div className="flex flex-col md:flex-row justify-between items-center gap-6">
          <div className="text-white/60 font-medium">
            © 2024 CodeMux Lab
          </div>
          <div className="flex gap-8">
            <a
              href="https://github.com/codemuxlab/codemux-cli"
              className="text-white/60 hover:text-white transition-colors duration-300 font-medium"
            >
              GitHub
            </a>
            <a
              href="https://github.com/codemuxlab/codemux-cli/issues"
              className="text-white/60 hover:text-white transition-colors duration-300 font-medium"
            >
              Issues
            </a>
            <a
              href="https://github.com/codemuxlab/codemux-cli/releases"
              className="text-white/60 hover:text-white transition-colors duration-300 font-medium"
            >
              Releases
            </a>
          </div>
        </div>
      </div>
    </div>
  );
}
