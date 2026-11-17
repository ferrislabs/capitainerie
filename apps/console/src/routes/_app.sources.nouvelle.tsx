import { createFileRoute } from '@tanstack/react-router'
import { SourceNewPage } from '#/domain/sources/feature/source-new-page'

export const Route = createFileRoute('/_app/sources/nouvelle')({
	component: SourceNewPage,
})
