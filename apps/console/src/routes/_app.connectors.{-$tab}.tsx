import { createFileRoute, redirect } from '@tanstack/react-router'
import { ConnectorsPage } from '#/domain/connectors/feature/connectors-page'

export const Route = createFileRoute('/_app/connectors/{-$tab}')({
	validateSearch: (search): { connected?: string } =>
		typeof search.connected === 'string' ? { connected: search.connected } : {},
	beforeLoad: ({ params }) => {
		if (params.tab === undefined)
			throw redirect({
				to: '/connectors/{-$tab}',
				params: { tab: 'installes' },
				replace: true,
			})
	},
	component: ConnectorsRoute,
})

function ConnectorsRoute() {
	const { tab } = Route.useParams()
	const { connected } = Route.useSearch()
	return <ConnectorsPage tab={tab} connectedId={connected} />
}
