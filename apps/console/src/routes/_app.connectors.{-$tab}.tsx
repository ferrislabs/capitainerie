import { createFileRoute, redirect } from '@tanstack/react-router'
import { ConnectorsPage } from '#/domain/connectors/feature/connectors-page'

export const Route = createFileRoute('/_app/connectors/{-$tab}')({
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
	return <ConnectorsPage tab={tab} />
}
