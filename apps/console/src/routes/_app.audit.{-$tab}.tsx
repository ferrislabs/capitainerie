import { createFileRoute } from '@tanstack/react-router'
import { AuditPage } from '#/domain/audit/feature/audit-page'
import { parseAuditTab } from '#/domain/audit/tabs'

export const Route = createFileRoute('/_app/audit/{-$tab}')({
	component: AuditRoute,
})

function AuditRoute() {
	const { tab } = Route.useParams()
	return <AuditPage tab={parseAuditTab(tab)} />
}
