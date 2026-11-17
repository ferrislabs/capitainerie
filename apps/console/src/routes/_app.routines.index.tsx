import { createFileRoute } from '@tanstack/react-router'
import { RoutinesPage } from '#/domain/routines/feature/routines-page'

export const Route = createFileRoute('/_app/routines/')({
	component: RoutinesPage,
})
