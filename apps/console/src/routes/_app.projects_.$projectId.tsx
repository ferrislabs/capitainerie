import { createFileRoute } from '@tanstack/react-router'
import { ProjectDetailPage } from '#/domain/clients/feature/project-detail-page'

export const Route = createFileRoute('/_app/projects_/$projectId')({
	component: ProjectDetailRoute,
})

function ProjectDetailRoute() {
	const { projectId } = Route.useParams()
	return <ProjectDetailPage projectId={projectId} />
}
