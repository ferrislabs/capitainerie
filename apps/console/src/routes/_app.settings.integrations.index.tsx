import { createFileRoute } from '@tanstack/react-router'
import { IntegrationsPage } from '#/domain/integrations/feature/integrations-page'

export const Route = createFileRoute('/_app/settings/integrations/')({
	validateSearch: (
		search: Record<string, unknown>,
	): { connected?: true; app?: 'created' } => ({
		...(search.connected === true || search.connected === 'true'
			? { connected: true as const }
			: {}),
		...(search.app === 'created' ? { app: 'created' as const } : {}),
	}),
	component: IntegrationsRoute,
})

function IntegrationsRoute() {
	const { connected, app } = Route.useSearch()
	return (
		<IntegrationsPage
			justConnected={connected === true}
			appCreated={app === 'created'}
		/>
	)
}
