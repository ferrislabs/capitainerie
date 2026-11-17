import {
	createFileRoute,
	Outlet,
	useLocation,
	useNavigate,
	useRouter,
} from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { AuthGate } from '#/components/auth-gate'
import { AppRail } from '#/components/layout/app-rail'
import { AppTopbar } from '#/components/layout/app-topbar'
import { breadcrumbs, type EntityNames } from '#/components/layout/breadcrumbs'
import { CommandPalette } from '#/components/layout/command-palette'
import { NAV_SECTIONS } from '#/components/layout/nav-items'
import { Notice, useNotice } from '#/components/layout/notice'
import type { PaletteCommand } from '#/components/layout/palette-filter'
import { useCommandPalette } from '#/components/layout/use-command-palette'
import { TooltipProvider } from '#/components/ui/tooltip'
import { shortName, useDemoState } from '#/domain/demo'
import { UserMenuFeature } from '#/domain/identity/feature/account-features'
import { useRoutinesState } from '#/domain/routines/feature/routines-store'
import { NewTaskDialog } from '#/domain/tasks/feature/new-task-dialog'
import { useMediaQuery } from '#/hooks/use-media-query'
import { usePersistedFlag } from '#/hooks/use-persisted-flag'
import { useRealtime } from '#/hooks/use-realtime'
import { useTheme } from '#/hooks/use-theme'
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
	const agents = useDemoState((s) => s.agents)
	const tasks = useDemoState((s) => s.tasks)
	const sources = useDemoState((s) => s.sources)
	const approvals = useDemoState((s) => s.approvals)
	const routines = useRoutinesState((s) => s.routines)
	const router = useRouter()
	const [notice, notify] = useNotice()
	const liveAgents = useMemo(
		() =>
			agents
				.filter((a) => a.status === 'running')
				.map((a) => ({ id: a.id, name: shortName(a.name), hue: a.hue })),
		[agents],
	)
	const names = useMemo<EntityNames>(
		() => ({
			task: (id) => tasks.find((t) => t.id === id)?.title,
			agent: (id) => agents.find((a) => a.id === id)?.name,
			source: (id) => sources.find((x) => x.id === id)?.name,
			routine: (id) => routines.find((r) => r.id === id)?.name,
		}),
		[tasks, agents, sources, routines],
	)
	const badges = useMemo(
		() => ({
			'/inbox/{-$tab}': 67,
			'/sources': 3,
			'/approvals': approvals.filter((a) => a.status === 'pending').length,
		}),
		[approvals],
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
						badges={badges}
						onOrganization={() =>
							notify("Changement d'organisation : une seule organisation")
						}
						footer={(isExpanded) => <UserMenuFeature compact={!isExpanded} />}
					/>
					<div className="flex min-w-0 flex-1 flex-col">
						<AppTopbar
							crumbs={breadcrumbs(pathname, names)}
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
				<Notice message={notice} className="fixed right-4 bottom-4 z-50" />
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
