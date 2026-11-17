import { createFileRoute } from '@tanstack/react-router'
import { DashboardPage } from '#/domain/dashboard/feature/dashboard-page'

export const Route = createFileRoute('/_app/dashboard')({
	component: DashboardPage,
})
