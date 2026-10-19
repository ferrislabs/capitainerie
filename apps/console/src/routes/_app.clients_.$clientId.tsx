import { createFileRoute } from '@tanstack/react-router'
import { ClientDetailPage } from '#/domain/clients/feature/client-detail-page'

export const Route = createFileRoute('/_app/clients_/$clientId')({
	component: ClientDetailRoute,
})

function ClientDetailRoute() {
	const { clientId } = Route.useParams()
	return <ClientDetailPage clientId={clientId} />
}
