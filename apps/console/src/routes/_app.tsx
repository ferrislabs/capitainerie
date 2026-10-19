import { createFileRoute, Outlet } from '@tanstack/react-router'
import { AuthGate } from '#/components/auth-gate'
import { AppRail, type RailBadges } from '#/components/layout/app-rail'
import { NAV_SECTIONS } from '#/components/layout/nav-items'
import { RealtimeIndicator } from '#/components/layout/realtime-indicator'
import { TooltipProvider } from '#/components/ui/tooltip'
import { UserMenuFeature } from '#/domain/identity/feature/account-features'
import { useActiveRunCount } from '#/domain/runs/hooks/use-active-runs'
import { useMediaQuery } from '#/hooks/use-media-query'
import { usePersistedFlag } from '#/hooks/use-persisted-flag'
import { useRealtime } from '#/hooks/use-realtime'

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

	return (
		<AuthGate>
			<TooltipProvider>
				<div className="flex min-h-dvh">
					<CountedRail
						sections={NAV_SECTIONS}
						expanded={expanded}
						canExpand={canExpand}
						onExpandedChange={setExpanded}
						footer={(isExpanded) => (
							<>
								<RealtimeIndicator compact={!isExpanded} />
								<UserMenuFeature compact={!isExpanded} />
							</>
						)}
					/>
					<main className="flex min-w-0 flex-1 flex-col">
						<Outlet />
					</main>
				</div>
			</TooltipProvider>
		</AuthGate>
	)
}
