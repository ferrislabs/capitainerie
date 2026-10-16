import type { QueryClient } from '@tanstack/react-query'
import {
	createRootRouteWithContext,
	HeadContent,
	Scripts,
} from '@tanstack/react-router'
import { ConfigGate } from '#/components/config-gate'
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
			{ name: 'theme-color', content: '#0f3d36' },
		],
		links: [
			{ rel: 'stylesheet', href: appCss },
			{ rel: 'icon', type: 'image/svg+xml', href: '/icon.svg' },
		],
	}),
	shellComponent: RootDocument,
})

function RootDocument({ children }: { children: React.ReactNode }) {
	return (
		<html lang="fr" translate="no">
			<head>
				<HeadContent />
			</head>
			<body>
				<ConfigGate>{children}</ConfigGate>
				<Scripts />
			</body>
		</html>
	)
}
