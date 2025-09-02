import Link from 'next/link';
import { ChevronRight, Terminal, Zap, Shield, Globe } from 'lucide-react';

export default function HomePage() {
  return (
    <main className="flex flex-1 flex-col">
      {/* Hero Section */}
      <section className="flex flex-col items-center justify-center py-20 text-center">
        <div className="mb-4 inline-flex items-center rounded-lg bg-fd-muted px-3 py-1 text-sm">
          <span className="text-fd-muted-foreground">🚀 Now with React Native support</span>
        </div>
        
        <h1 className="mb-6 text-5xl font-bold tracking-tight md:text-6xl">
          Terminal Multiplexer for{' '}
          <span className="bg-gradient-to-r from-blue-600 to-purple-600 bg-clip-text text-transparent">
            AI Coding
          </span>
        </h1>
        
        <p className="mb-8 max-w-2xl text-lg text-fd-muted-foreground">
          CodeMux is a specialized terminal multiplexer designed for "vibe coding" - 
          letting AI assistants like Claude, Gemini, and Aider drive your development 
          with enhanced web UI support.
        </p>
        
        <div className="flex flex-col gap-4 sm:flex-row">
          <Link
            href="/docs"
            className="inline-flex items-center justify-center rounded-lg bg-fd-primary px-6 py-3 text-sm font-medium text-fd-primary-foreground transition-colors hover:bg-fd-primary/90"
          >
            Get Started
            <ChevronRight className="ml-2 h-4 w-4" />
          </Link>
          <a
            href="https://github.com/codemuxlab/codemux-cli"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center justify-center rounded-lg border border-fd-border px-6 py-3 text-sm font-medium transition-colors hover:bg-fd-muted"
          >
            View on GitHub
          </a>
        </div>
      </section>

      {/* Features Section */}
      <section className="border-t border-fd-border py-20">
        <div className="mx-auto max-w-6xl px-6">
          <h2 className="mb-12 text-center text-3xl font-bold">Why CodeMux?</h2>
          
          <div className="grid gap-8 md:grid-cols-2 lg:grid-cols-4">
            <FeatureCard
              icon={<Shield className="h-6 w-6" />}
              title="Secure by Design"
              description="Only runs whitelisted AI CLI tools. Your code stays safe while AI assists."
            />
            <FeatureCard
              icon={<Zap className="h-6 w-6" />}
              title="Smart Prompts"
              description="Detects and intercepts interactive prompts with native web UI components."
            />
            <FeatureCard
              icon={<Globe className="h-6 w-6" />}
              title="Web & Mobile"
              description="Rich web interfaces and React Native support for coding on any device."
            />
            <FeatureCard
              icon={<Terminal className="h-6 w-6" />}
              title="True Multiplexing"
              description="Manage multiple AI sessions across different projects simultaneously."
            />
          </div>
        </div>
      </section>

      {/* Quick Start Section */}
      <section className="border-t border-fd-border py-20">
        <div className="mx-auto max-w-4xl px-6">
          <h2 className="mb-8 text-center text-3xl font-bold">Quick Start</h2>
          
          <div className="rounded-lg border border-fd-border bg-fd-card p-6">
            <h3 className="mb-4 text-lg font-semibold">Install with Homebrew</h3>
            <pre className="mb-4 overflow-x-auto rounded-md bg-fd-muted p-4">
              <code className="text-sm">brew install codemuxlab/tap/codemux</code>
            </pre>
            
            <h3 className="mb-4 text-lg font-semibold">Run your AI assistant</h3>
            <pre className="overflow-x-auto rounded-md bg-fd-muted p-4">
              <code className="text-sm">{`# Quick mode - launch immediately
codemux run claude

# Server mode - manage multiple sessions
codemux server start
codemux run claude --open  # Opens web UI`}</code>
            </pre>
          </div>
          
          <div className="mt-8 text-center">
            <Link
              href="/docs"
              className="inline-flex items-center text-fd-primary hover:underline"
            >
              View full documentation
              <ChevronRight className="ml-1 h-4 w-4" />
            </Link>
          </div>
        </div>
      </section>

      {/* Operating Modes */}
      <section className="border-t border-fd-border py-20">
        <div className="mx-auto max-w-6xl px-6">
          <h2 className="mb-12 text-center text-3xl font-bold">Operating Modes</h2>
          
          <div className="grid gap-8 md:grid-cols-2">
            <div className="rounded-lg border border-fd-border p-6">
              <h3 className="mb-3 text-xl font-semibold">Quick Mode</h3>
              <p className="mb-4 text-fd-muted-foreground">
                Launch a single AI session immediately. Perfect for quick tasks and focused work.
              </p>
              <pre className="rounded-md bg-fd-muted p-3">
                <code className="text-sm">codemux run claude</code>
              </pre>
            </div>
            
            <div className="rounded-lg border border-fd-border p-6">
              <h3 className="mb-3 text-xl font-semibold">Server Mode</h3>
              <p className="mb-4 text-fd-muted-foreground">
                Background service managing multiple project sessions. Ideal for complex workflows.
              </p>
              <pre className="rounded-md bg-fd-muted p-3">
                <code className="text-sm">codemux server start</code>
              </pre>
            </div>
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section className="border-t border-fd-border py-20">
        <div className="mx-auto max-w-4xl px-6 text-center">
          <h2 className="mb-4 text-3xl font-bold">Ready to enhance your AI coding?</h2>
          <p className="mb-8 text-lg text-fd-muted-foreground">
            Join developers who are embracing the future of AI-assisted development.
          </p>
          <div className="flex flex-col gap-4 sm:flex-row sm:justify-center">
            <Link
              href="/docs"
              className="inline-flex items-center justify-center rounded-lg bg-fd-primary px-6 py-3 text-sm font-medium text-fd-primary-foreground transition-colors hover:bg-fd-primary/90"
            >
              Read Documentation
              <ChevronRight className="ml-2 h-4 w-4" />
            </Link>
            <a
              href="https://github.com/codemuxlab/codemux-cli/releases"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center justify-center rounded-lg border border-fd-border px-6 py-3 text-sm font-medium transition-colors hover:bg-fd-muted"
            >
              Download Latest Release
            </a>
          </div>
        </div>
      </section>
    </main>
  );
}

function FeatureCard({ icon, title, description }: {
  icon: React.ReactNode;
  title: string;
  description: string;
}) {
  return (
    <div className="group rounded-lg border border-fd-border p-6 transition-colors hover:border-fd-primary/50">
      <div className="mb-3 inline-flex rounded-lg bg-fd-primary/10 p-2 text-fd-primary">
        {icon}
      </div>
      <h3 className="mb-2 font-semibold">{title}</h3>
      <p className="text-sm text-fd-muted-foreground">{description}</p>
    </div>
  );
}