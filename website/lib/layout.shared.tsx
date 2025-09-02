import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';

/**
 * Shared layout configurations
 *
 * you can customise layouts individually from:
 * Home Layout: app/(home)/layout.tsx
 * Docs Layout: app/docs/layout.tsx
 */
export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: (
        <>
          <svg
            width="24"
            height="24"
            xmlns="http://www.w3.org/2000/svg"
            aria-label="Logo"
          >
            <title>CodeMux Logo</title>
            <circle cx={12} cy={12} r={12} fill="currentColor" />
          </svg>
          CodeMux
        </>
      ),
    },
    // see https://fumadocs.dev/docs/ui/navigation/links
    links: [
      {
        text: 'Documentation',
        url: '/docs',
      },
      {
        text: 'Releases',
        url: 'https://github.com/codemuxlab/codemux-cli/releases',
        external: true,
      },
    ],
    githubUrl: 'https://github.com/codemuxlab/codemux-cli'
  };
}
