import { createFileRoute } from '@tanstack/react-router'
import { ProjectsPage } from '#/domain/clients/feature/projects-page'

export const Route = createFileRoute('/_app/projects')({
	component: ProjectsPage,
})
