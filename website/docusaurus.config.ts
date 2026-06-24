import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'spec-spine',
  tagline: 'Frozen, hash-verifiable specs as the unit of governance for your code',
  favicon: 'img/favicon.ico',

  // For GitHub Pages: custom domain later -> baseUrl: '/' plus static/CNAME
  url: 'https://stagecraft-ing.github.io',
  baseUrl: '/spec-spine/',

  organizationName: 'stagecraft-ing',
  projectName: 'spec-spine',

  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  markdown: {
    mermaid: true,
  },

  themes: ['@docusaurus/theme-mermaid'],

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          editUrl:
            'https://github.com/stagecraft-ing/spec-spine/tree/main/website/',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    navbar: {
      title: 'spec-spine',
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docsSidebar',
          position: 'left',
          label: 'Docs',
        },
        {
          href: 'https://github.com/stagecraft-ing/spec-spine',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            {
              label: 'Getting Started',
              to: '/docs/getting-started/installation',
            },
            {
              label: 'CLI Reference',
              to: '/docs/cli/overview',
            },
            {
              label: 'Configuration',
              to: '/docs/configuration',
            },
          ],
        },
        {
          title: 'Community',
          items: [
            {
              label: 'GitHub',
              href: 'https://github.com/stagecraft-ing/spec-spine',
            },
            {
              label: 'crates.io',
              href: 'https://crates.io/crates/spec-spine-cli',
            },
            {
              label: 'npm',
              href: 'https://www.npmjs.com/package/spec-spine',
            },
          ],
        },
        {
          title: 'More',
          items: [
            {
              label: 'Adoption Kit (spec-spine + Claude Code)',
              href: '/spec-spine/docs/adoption-guide',
            },
          ],
        },
      ],
      copyright: `Copyright ${new Date().getFullYear()} The spec-spine Authors. Apache-2.0.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['rust', 'toml', 'bash', 'json'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
