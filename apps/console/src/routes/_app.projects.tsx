import { createFileRoute } from '@tanstack/react-router'
import { ProjectsPage } from '#/domain/projects/feature/projects-page'

export const Route = createFileRoute('/_app/projects')({
	component: ProjectsPage,
})
