import { createFileRoute, Outlet } from '@tanstack/react-router'
import { Page } from '#/components/layout/page'

export const Route = createFileRoute('/_app/settings')({
	component: SettingsLayout,
})

function SettingsLayout() {
	return (
		<Page className="max-w-5xl">
			<Outlet />
		</Page>
	)
}
