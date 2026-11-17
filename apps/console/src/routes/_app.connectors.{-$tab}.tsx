import { createFileRoute } from '@tanstack/react-router'
import { ConnectorsPage } from '#/domain/connectors/feature/connectors-page'

export const Route = createFileRoute('/_app/connectors/{-$tab}')({
	component: ConnectorsRoute,
})

function ConnectorsRoute() {
	const { tab } = Route.useParams()
	return <ConnectorsPage tab={tab} />
}
