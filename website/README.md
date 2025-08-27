# CodeMux Website

Marketing and documentation website for [CodeMux](https://www.codemux.dev) - terminal multiplexer for AI coding CLIs with mobile-ready React Native UI.

## Overview

This is the public-facing website for CodeMux, built with Next.js. It serves as:
- Landing page showcasing CodeMux features and animated architecture diagram
- Installation guide with multiple methods (Homebrew, npm, shell scripts)
- Usage examples and quick start instructions
- Documentation hub for vibe coding workflows

## Development

### Prerequisites
- Node.js 18+
- npm or yarn

### Setup

```bash
# Install dependencies
npm install

# Run development server
npm run dev

# IMPORTANT: Always lint before committing
npm run lint
```

Open [http://localhost:3000](http://localhost:3000) to view the site.

### Project Structure

```
website/
├── src/
│   └── app/          # Next.js App Router pages
├── public/           # Static assets
└── package.json      # Dependencies and scripts
```

## Deployment

The website is deployed on Vercel and automatically updates when changes are pushed to the main branch.

### Production URL
- Main: [https://www.codemux.dev](https://www.codemux.dev)

## Content Updates

- **Landing Page**: Edit `src/app/page.tsx`
- **Global Styles**: Edit `src/app/globals.css`
- **Site Layout**: Edit `src/app/layout.tsx`

## Tech Stack

- **Framework**: Next.js 15 with App Router and Turbopack
- **Styling**: Tailwind CSS v4
- **Animations**: MagicUI AnimatedBeam components
- **Deployment**: Vercel
- **Font**: Geist (Vercel's font family)
- **Linting**: Biome for fast TypeScript/JavaScript linting

## Related Projects

- **Main Repository**: [codemux-cli](https://github.com/codemuxlab/codemux-cli) - The core CodeMux application
- **React Native App**: Located in `../app/` - The cross-platform UI for CodeMux
