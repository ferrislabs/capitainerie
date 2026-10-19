import { createFileRoute } from '@tanstack/react-router'
import { ClientsPage } from '#/domain/clients/feature/clients-page'

export const Route = createFileRoute('/_app/clients')({
	component: ClientsPage,
})
