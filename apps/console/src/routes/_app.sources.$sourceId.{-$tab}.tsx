import { createFileRoute } from '@tanstack/react-router'
import { SourceDetailPage } from '#/domain/sources/feature/source-detail-page'

export const Route = createFileRoute('/_app/sources/$sourceId/{-$tab}')({
	component: SourceDetailRoute,
})

function SourceDetailRoute() {
	const { sourceId, tab } = Route.useParams()
	return <SourceDetailPage sourceId={sourceId} tab={tab} />
}
