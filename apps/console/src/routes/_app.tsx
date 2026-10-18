import { createFileRoute, Outlet } from '@tanstack/react-router'
import { AuthGate } from '#/components/auth-gate'
import { AppRail } from '#/components/layout/app-rail'
import { PRIMARY_NAV, SECONDARY_NAV } from '#/components/layout/nav-items'
import { RealtimeIndicator } from '#/components/layout/realtime-indicator'
import { ThemeToggle } from '#/components/layout/theme-toggle'
import { TopBar } from '#/components/layout/top-bar'
import { TooltipProvider } from '#/components/ui/tooltip'
import {
	OrganizationNameFeature,
	UserMenuFeature,
} from '#/domain/identity/feature/account-features'
import { useMediaQuery } from '#/hooks/use-media-query'
import { usePersistedFlag } from '#/hooks/use-persisted-flag'
import { useRealtime } from '#/hooks/use-realtime'
import { useTheme } from '#/hooks/use-theme'

export const Route = createFileRoute('/_app')({ component: AppLayout })

const RAIL_STORAGE_KEY = 'capitainerie.rail-expanded'

function AppLayout() {
	useRealtime()
	const theme = useTheme()
	const [expanded, setExpanded] = usePersistedFlag(RAIL_STORAGE_KEY, true)
	const canExpand = useMediaQuery('(min-width: 1024px)')

	return (
		<AuthGate>
			<TooltipProvider>
				<div className="flex min-h-dvh">
					<AppRail
						primary={PRIMARY_NAV}
						secondary={SECONDARY_NAV}
						expanded={expanded}
						canExpand={canExpand}
						onExpandedChange={setExpanded}
					/>
					<div className="flex min-w-0 flex-1 flex-col">
						<TopBar
							organization={<OrganizationNameFeature />}
							actions={
								<>
									<RealtimeIndicator />
									<ThemeToggle
										preference={theme.preference}
										onChange={theme.setPreference}
									/>
									<UserMenuFeature />
								</>
							}
						/>
						<main className="flex min-w-0 flex-1 flex-col">
							<Outlet />
						</main>
					</div>
				</div>
			</TooltipProvider>
		</AuthGate>
	)
}
