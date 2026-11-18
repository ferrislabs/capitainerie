import { createFileRoute } from '@tanstack/react-router'
import { ProvidersPage } from '#/domain/providers/feature/providers-page'

export const Route = createFileRoute('/_app/providers')({
	component: ProvidersPage,
})
