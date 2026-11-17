import { createFileRoute } from '@tanstack/react-router'
import { RoutineNewPage } from '#/domain/routines/feature/routine-new-page'

export const Route = createFileRoute('/_app/routines/nouvelle')({
	component: RoutineNewPage,
})
