import { createFileRoute } from '@tanstack/react-router'
import { RoutineDetailPage } from '#/domain/routines/feature/routine-detail-page'

export const Route = createFileRoute('/_app/routines/$routineId/{-$tab}')({
	component: RoutineDetailRoute,
})

function RoutineDetailRoute() {
	const { routineId, tab } = Route.useParams()
	return <RoutineDetailPage routineId={routineId} tab={tab} />
}
