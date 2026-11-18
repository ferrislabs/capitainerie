import {
	createFileRoute,
	Outlet,
	useLocation,
	useNavigate,
	useRouter,
} from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { AuthGate } from '#/components/auth-gate'
import { AppRail, type RailBadges } from '#/components/layout/app-rail'
import { AppTopbar } from '#/components/layout/app-topbar'
import { breadcrumbs, type EntityNames } from '#/components/layout/breadcrumbs'
import { CommandPalette } from '#/components/layout/command-palette'
import { NAV_SECTIONS } from '#/components/layout/nav-items'
import type { PaletteCommand } from '#/components/layout/palette-filter'
import { useCommandPalette } from '#/components/layout/use-command-palette'
import { TooltipProvider } from '#/components/ui/tooltip'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { usePendingApprovalsCount } from '#/domain/approvals/hooks/use-approvals'
import { useActiveRuns } from '#/domain/dashboard/hooks/use-dashboard-data'
import { UserMenuFeature } from '#/domain/identity/feature/account-features'
import { useMe } from '#/domain/identity/hooks/use-me'
import { NewTaskDialog } from '#/domain/tasks/feature/new-task-dialog'
import { useCards } from '#/domain/tasks/hooks/use-tasks'
import { useMediaQuery } from '#/hooks/use-media-query'
import { usePersistedFlag } from '#/hooks/use-persisted-flag'
import { useRealtime } from '#/hooks/use-realtime'
import { useTheme } from '#/hooks/use-theme'
import { shortName } from '#/lib/agent-name'
import { resolveTheme, systemPrefersDark } from '#/lib/theme'

export const Route = createFileRoute('/_app')({ component: AppLayout })

const RAIL_STORAGE_KEY = 'capitainerie.rail-expanded'

function AppLayout() {
	useRealtime()
	const [expanded, setExpanded] = usePersistedFlag(RAIL_STORAGE_KEY, true)
	const canExpand = useMediaQuery('(min-width: 1024px)')
	const { pathname } = useLocation()
	const navigate = useNavigate()
	const { open, setOpen } = useCommandPalette()
	const [newTaskOpen, setNewTaskOpen] = useState(false)
	const agents = useTeamAgents().data?.items
	const cards = useCards().data?.items
	const liveCount = useActiveRuns().data?.items.length ?? 0
	const organization = useMe().data?.organization.name
	const pendingApprovals = usePendingApprovalsCount()
	const badges = useMemo<RailBadges>(
		() => (pendingApprovals > 0 ? { '/approvals': pendingApprovals } : {}),
		[pendingApprovals],
	)
	const router = useRouter()
	const liveAgents = useMemo(
		() =>
			(agents ?? [])
				.filter((a) => a.status === 'running')
				.map((a) => ({ id: a.id, name: shortName(a.name), hue: a.hue })),
		[agents],
	)
	const names = useMemo<EntityNames>(
		() => ({
			task: (id) => cards?.find((t) => t.id === id)?.title,
			agent: (id) => agents?.find((a) => a.id === id)?.name,
		}),
		[cards, agents],
	)
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
				id: 'new-task',
				group: 'Action',
				label: 'Nouvelle tâche',
				run: () => setNewTaskOpen(true),
			},
			{
				id: 'new-routine',
				group: 'Action',
				label: 'Nouvelle routine',
				to: '/routines/nouvelle',
			},
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
					<AppRail
						sections={NAV_SECTIONS}
						expanded={expanded}
						canExpand={canExpand}
						onExpandedChange={setExpanded}
						onSearch={() => setOpen(true)}
						onNewTask={() => setNewTaskOpen(true)}
						liveAgents={liveAgents}
						liveCount={liveCount}
						badges={badges}
						organization={organization}
						footer={(isExpanded) => <UserMenuFeature compact={!isExpanded} />}
					/>
					<div className="flex min-w-0 flex-1 flex-col">
						<AppTopbar
							crumbs={breadcrumbs(pathname, names)}
							organization={organization}
							liveCount={liveCount}
							onSearch={() => setOpen(true)}
							dark={dark}
							onToggleTheme={() => setPreference(dark ? 'light' : 'dark')}
							onRefresh={() => void router.invalidate()}
							showRefresh={!pathname.startsWith('/tasks/')}
						/>
						<main className="flex min-w-0 flex-1 flex-col">
							<Outlet />
						</main>
					</div>
				</div>
				<NewTaskDialog open={newTaskOpen} onOpenChange={setNewTaskOpen} />
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
