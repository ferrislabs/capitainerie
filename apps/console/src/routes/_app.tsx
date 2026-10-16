import { createFileRoute, Outlet } from '@tanstack/react-router'
import { AuthGate } from '#/components/auth-gate'
import { AppSidebar } from '#/components/layout/app-sidebar'

export const Route = createFileRoute('/_app')({ component: AppLayout })

function AppLayout() {
	return (
		<AuthGate>
			<div className="flex min-h-screen">
				<AppSidebar />
				<main className="flex-1 p-6">
					<Outlet />
				</main>
			</div>
		</AuthGate>
	)
}
