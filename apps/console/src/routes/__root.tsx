import '@fontsource-variable/atkinson-hyperlegible-next'
import '@fontsource-variable/atkinson-hyperlegible-mono'
import type { QueryClient } from '@tanstack/react-query'
import {
	createRootRouteWithContext,
	HeadContent,
	Scripts,
} from '@tanstack/react-router'
import { ConfigGate } from '#/components/config-gate'
import { useThemeSync } from '#/hooks/use-theme'
import { THEME_BOOT_SCRIPT } from '#/lib/theme'
import appCss from '#/styles.css?url'

interface RouterContext {
	queryClient: QueryClient
}

export const Route = createRootRouteWithContext<RouterContext>()({
	head: () => ({
		meta: [
			{ charSet: 'utf-8' },
			{ name: 'viewport', content: 'width=device-width, initial-scale=1' },
			{ name: 'google', content: 'notranslate' },
			{ title: 'Capitainerie · Console' },
			{ name: 'theme-color', content: '#0f233f' },
		],
		links: [
			{ rel: 'stylesheet', href: appCss },
			{ rel: 'icon', type: 'image/svg+xml', href: '/icon.svg' },
		],
		scripts: [{ children: THEME_BOOT_SCRIPT }],
	}),
	shellComponent: RootDocument,
})

function ThemeSync() {
	useThemeSync()
	return null
}

function RootDocument({ children }: { children: React.ReactNode }) {
	return (
		<html lang="fr" translate="no" suppressHydrationWarning>
			<head>
				<HeadContent />
			</head>
			<body>
				<ThemeSync />
				<ConfigGate>{children}</ConfigGate>
				<Scripts />
			</body>
		</html>
	)
}
