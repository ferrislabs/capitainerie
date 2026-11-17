import {
	createFileRoute,
	Outlet,
	useLocation,
	useNavigate,
} from '@tanstack/react-router'
import { useMemo } from 'react'
import { AuthGate } from '#/components/auth-gate'
import { AppRail, type RailBadges } from '#/components/layout/app-rail'
import { AppTopbar } from '#/components/layout/app-topbar'
import { breadcrumbs } from '#/components/layout/breadcrumbs'
import { CommandPalette } from '#/components/layout/command-palette'
import { NAV_SECTIONS } from '#/components/layout/nav-items'
import type { PaletteCommand } from '#/components/layout/palette-filter'
import { RealtimeIndicator } from '#/components/layout/realtime-indicator'
import { useCommandPalette } from '#/components/layout/use-command-palette'
import { TooltipProvider } from '#/components/ui/tooltip'
import { UserMenuFeature } from '#/domain/identity/feature/account-features'
import { useActiveRunCount } from '#/domain/runs/hooks/use-active-runs'
import { useMediaQuery } from '#/hooks/use-media-query'
import { usePersistedFlag } from '#/hooks/use-persisted-flag'
import { useRealtime } from '#/hooks/use-realtime'
import { useTheme } from '#/hooks/use-theme'
import { resolveTheme, systemPrefersDark } from '#/lib/theme'

export const Route = createFileRoute('/_app')({ component: AppLayout })

const RAIL_STORAGE_KEY = 'capitainerie.rail-expanded'

function CountedRail(props: Parameters<typeof AppRail>[0]) {
	const active = useActiveRunCount()
	const badges: RailBadges = active === undefined ? {} : { '/live': active }
	return <AppRail {...props} badges={badges} />
}

function AppLayout() {
	useRealtime()
	const [expanded, setExpanded] = usePersistedFlag(RAIL_STORAGE_KEY, true)
	const canExpand = useMediaQuery('(min-width: 1024px)')
	const { pathname } = useLocation()
	const navigate = useNavigate()
	const { open, setOpen } = useCommandPalette()
	const { preference, setPreference } = useTheme()
	const dark = resolveTheme(preference, systemPrefersDark()) === 'dark'

	const commands = useMemo<ReadonlyArray<PaletteCommand>>(
		() => [
			...NAV_SECTIONS.flatMap((section) =>
				section.items.map((item) => ({
					id: item.to,
					group: 'Aller à',
					label: item.label,
					icon: item.icon,
					to: item.to,
				})),
			),
			{
				id: 'theme',
				group: 'Action',
				label: 'Changer de thème',
				run: () => setPreference(dark ? 'light' : 'dark'),
			},
		],
		[dark, setPreference],
	)

	function runCommand(command: PaletteCommand) {
		if (command.to) void navigate({ to: command.to })
		else command.run?.()
	}

	return (
		<AuthGate>
			<TooltipProvider>
				<div className="flex min-h-dvh">
					<CountedRail
						sections={NAV_SECTIONS}
						expanded={expanded}
						canExpand={canExpand}
						onExpandedChange={setExpanded}
						onSearch={() => setOpen(true)}
						footer={(isExpanded) => <UserMenuFeature compact={!isExpanded} />}
					/>
					<div className="flex min-w-0 flex-1 flex-col">
						<AppTopbar
							crumbs={breadcrumbs(pathname)}
							onSearch={() => setOpen(true)}
							dark={dark}
							onToggleTheme={() => setPreference(dark ? 'light' : 'dark')}
							trailing={<RealtimeIndicator />}
						/>
						<main className="flex min-w-0 flex-1 flex-col">
							<Outlet />
						</main>
					</div>
				</div>
				<CommandPalette
					open={open}
					onOpenChange={setOpen}
					commands={commands}
					onRun={runCommand}
				/>
			</TooltipProvider>
		</AuthGate>
	)
}
