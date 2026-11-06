import { defineConfig } from 'vitepress'

// GitHub Pages serves a project site under /<repository>/; the deploy workflow
// sets DOCS_BASE from the repository name. Locally the site lives at /.
const base = process.env.DOCS_BASE || '/'

export default defineConfig({
  base,
  title: 'keyhop',
  description:
    'App hotkeys, smooth scrolling, encrypted keyboard/mouse sharing, file transfer and keyboard settings in one small native binary.',
  lang: 'en',
  cleanUrls: true,
  lastUpdated: false,
  srcExclude: ['**/node_modules/**'],
  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: `${base}logo.svg` }],
    ['meta', { name: 'theme-color', content: '#7c6cf5' }],
  ],
  themeConfig: {
    logo: '/logo.svg',
    nav: [
      { text: 'Guide', link: '/guide/getting-started', activeMatch: '/guide/' },
      { text: 'Features', link: '/features/hotkeys', activeMatch: '/features/' },
      { text: 'Reference', link: '/reference/configuration', activeMatch: '/reference/' },
      { text: 'Hardware', link: '/hardware/keychron-v1-ultra-8k', activeMatch: '/hardware/' },
    ],
    sidebar: [
      {
        text: 'Guide',
        items: [
          { text: 'Getting started', link: '/guide/getting-started' },
          { text: 'Installation', link: '/guide/installation' },
          { text: 'Troubleshooting', link: '/guide/troubleshooting' },
        ],
      },
      {
        text: 'Features',
        items: [
          { text: 'App-launch hotkeys', link: '/features/hotkeys' },
          { text: 'Smooth scrolling', link: '/features/smooth-scrolling' },
          { text: 'Keyboard & mouse sharing', link: '/features/sharing' },
          { text: 'Keyboard settings', link: '/features/keyboard-settings' },
          { text: 'Accent-menu lights', link: '/features/accent-lights' },
          { text: 'Terminal hints', link: '/features/terminal-hints' },
        ],
      },
      {
        text: 'Reference',
        items: [
          { text: 'Configuration', link: '/reference/configuration' },
          { text: 'Commands', link: '/reference/commands' },
          { text: 'Key names', link: '/reference/key-names' },
          { text: 'Security model', link: '/reference/security' },
        ],
      },
      {
        text: 'Hardware',
        items: [{ text: 'Keychron V1 Ultra 8K', link: '/hardware/keychron-v1-ultra-8k' }],
      },
    ],
    socialLinks: [{ icon: 'github', link: 'https://github.com/miercolesv/keyhop' }],
    editLink: {
      pattern: 'https://github.com/miercolesv/keyhop/edit/main/docs/:path',
      text: 'Edit this page on GitHub',
    },
    search: { provider: 'local' },
    outline: { level: [2, 3], label: 'On this page' },
    footer: {
      message: 'Released under the MIT License. Not affiliated with any keyboard manufacturer.',
    },
  },
})
