import { createFileRoute } from '@tanstack/react-router'
import { ArtifactsPage } from '#/domain/artifacts/feature/artifacts-page'

export const Route = createFileRoute('/_app/artifacts')({
	component: ArtifactsPage,
})
