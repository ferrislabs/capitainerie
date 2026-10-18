import { createFileRoute } from '@tanstack/react-router'
import { ModelsPage } from '#/domain/settings/feature/models-page'

export const Route = createFileRoute('/_app/settings/models')({
	component: ModelsPage,
})
