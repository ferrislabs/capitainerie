import { createFileRoute } from '@tanstack/react-router'
import { ArtifactPage } from '#/domain/artifacts/feature/artifact-page'

export const Route = createFileRoute('/_app/artifacts/$artifactId')({
	component: ArtifactRoute,
})

function ArtifactRoute() {
	const { artifactId } = Route.useParams()
	return <ArtifactPage artifactId={artifactId} />
}
