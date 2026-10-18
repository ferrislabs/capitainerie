import { createFileRoute } from '@tanstack/react-router'
import { IntegrationsPage } from '#/domain/integrations/feature/integrations-page'

export const Route = createFileRoute('/_app/settings/integrations/')({
	validateSearch: (search: Record<string, unknown>): { connected?: true } =>
		search.connected === true || search.connected === 'true'
			? { connected: true }
			: {},
	component: IntegrationsRoute,
})

function IntegrationsRoute() {
	const { connected } = Route.useSearch()
	return <IntegrationsPage justConnected={connected === true} />
}
