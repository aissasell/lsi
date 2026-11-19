// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	integrations: [
		starlight({
			title: 'lsi',
			description: 'Interactive eza-like terminal file explorer with Nerd Fonts and themes',
			social: [
				{
					icon: 'github',
					label: 'GitHub',
					href: 'https://github.com/aissasell/lsi',
				},
			],
			sidebar: [
				{
					label: 'Guides',
					items: [
						{ label: 'Getting Started', slug: 'guides/getting-started' },
						{ label: 'Navigation & Usage', slug: 'guides/usage' },
						{ label: 'Shell Integration', slug: 'guides/shell-integration' },
						{ label: 'Themes & Custom Palette', slug: 'guides/themes' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'CLI Options', slug: 'reference/cli' },
						{ label: 'Keybindings', slug: 'reference/keybindings' },
						{ label: 'Configuration (config.toml)', slug: 'reference/configuration' },
						{ label: 'Theme Reference (theme.toml)', slug: 'reference/theme-toml' },
					],
				},
			],
		}),
	],
});
