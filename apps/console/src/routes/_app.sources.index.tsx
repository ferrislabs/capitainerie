import { createFileRoute } from '@tanstack/react-router'
import { SourcesPage } from '#/domain/sources/feature/sources-page'

export const Route = createFileRoute('/_app/sources/')({
	component: SourcesPage,
})
