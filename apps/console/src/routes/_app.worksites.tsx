import { createFileRoute } from '@tanstack/react-router'
import { WorksitesPage } from '#/domain/worksites/feature/worksites-page'

export const Route = createFileRoute('/_app/worksites')({
	component: WorksitesPage,
})
