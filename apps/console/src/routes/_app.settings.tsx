import { createFileRoute, Outlet } from '@tanstack/react-router'
import { SettingsNav } from '#/components/layout/settings-nav'
import { SETTINGS_NAV } from '#/domain/settings/nav'

export const Route = createFileRoute('/_app/settings')({
	component: SettingsLayout,
})

function SettingsLayout() {
	return (
		<div className="flex min-w-0 flex-1 flex-col lg:flex-row">
			<SettingsNav
				title="Paramètres"
				groupLabel="Organisation"
				items={SETTINGS_NAV}
			/>
			<div className="mx-auto flex w-full max-w-5xl min-w-0 flex-col gap-5 px-4 py-5 sm:px-6 sm:py-6">
				<Outlet />
			</div>
		</div>
	)
}
