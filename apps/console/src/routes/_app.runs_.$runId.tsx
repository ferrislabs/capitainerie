import { createFileRoute } from '@tanstack/react-router'
import { RunDetailPage } from '#/domain/runs/feature/run-detail-page'

export const Route = createFileRoute('/_app/runs_/$runId')({
	component: RunDetailRoute,
})

function RunDetailRoute() {
	const { runId } = Route.useParams()
	return <RunDetailPage runId={runId} />
}
