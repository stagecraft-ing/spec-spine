import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    {
      type: 'category',
      label: 'Getting Started',
      items: [
        'getting-started/installation',
        'getting-started/quickstart',
      ],
    },
    {
      type: 'category',
      label: 'Concepts',
      items: [
        'concepts/overview',
        'concepts/edges-and-units',
        'concepts/derived-authority',
        'concepts/coupling-gate',
        'concepts/determinism',
        'concepts/constitutional-hierarchy',
        'concepts/waivers',
      ],
    },
    {
      type: 'category',
      label: 'CLI Reference',
      items: [
        'cli/overview',
        'cli/init',
        'cli/compile',
        'cli/index',
        'cli/registry',
        'cli/lint',
        'cli/couple',
      ],
    },
    {
      type: 'doc',
      id: 'configuration',
      label: 'Configuration',
    },
    {
      type: 'doc',
      id: 'adoption-guide',
      label: 'Adoption Guide',
    },
    {
      type: 'doc',
      id: 'api-reference',
      label: 'API Reference',
    },
    {
      type: 'doc',
      id: 'extending-and-overlays',
      label: 'Extending and Overlays',
    },
    {
      type: 'doc',
      id: 'schema-and-versioning',
      label: 'Schema and Versioning',
    },
    {
      type: 'doc',
      id: 'releasing',
      label: 'Releasing',
    },
    {
      type: 'doc',
      id: 'faq',
      label: 'FAQ and Troubleshooting',
    },
    {
      type: 'category',
      label: 'Use with Claude Code',
      items: [
        'claude-code/overview',
        'claude-code/install',
        'claude-code/session-init',
        'claude-code/governed-loop',
        'claude-code/skills',
        'claude-code/agents',
        'claude-code/rules',
        'claude-code/configuration',
        'claude-code/adopt-in-your-repo',
        'claude-code/troubleshooting',
      ],
    },
  ],
};

export default sidebars;
