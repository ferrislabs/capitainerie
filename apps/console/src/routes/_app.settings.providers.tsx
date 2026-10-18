import { createFileRoute } from '@tanstack/react-router'
import { ProvidersPage } from '#/domain/settings/feature/providers-page'

export const Route = createFileRoute('/_app/settings/providers')({
	component: ProvidersPage,
})
