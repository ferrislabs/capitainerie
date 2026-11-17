import { Link } from '@tanstack/react-router'
import { Play, Repeat } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { EmptyState } from '#/components/layout/empty-state'
import { Notice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { TabNav } from '#/components/layout/tab-nav'
import { Button } from '#/components/ui/button'
import { shortName, useDemoState } from '#/domain/demo'
import {
	launchRoutine,
	toggleRoutine,
	useRoutinesState,
} from '#/domain/routines/feature/routines-store'
import { SettingsTab } from '#/domain/routines/feature/settings-tab'
import { TriggersPanel } from '#/domain/routines/feature/triggers-panel'
import {
	useNow,
	useRoutineNotice,
} from '#/domain/routines/feature/use-routine-context'
import { scheduleLabel, upcomingRuns } from '#/domain/routines/routine-view'
import { AgentAvatar } from '#/domain/routines/ui/agent-avatar'
import { OverviewTab } from '#/domain/routines/ui/overview-tab'
import { RunsTab } from '#/domain/routines/ui/runs-tab'

export const ROUTINE_TABS = [
	{ key: 'apercu', label: "Vue d'ensemble" },
	{ key: 'declencheurs', label: 'Déclencheurs' },
	{ key: 'executions', label: 'Exécutions' },
	{ key: 'reglages', label: 'Réglages' },
] as const

export type RoutineTab = (typeof ROUTINE_TABS)[number]['key']

export const resolveTab = (tab: string | undefined): RoutineTab =>
	ROUTINE_TABS.find((t) => t.key === tab)?.key ?? 'apercu'

interface Props {
	routineId: string
	tab?: string | undefined
}

export function RoutineDetailPage({ routineId, tab }: Props) {
	const routine = useRoutinesState((s) =>
		s.routines.find((r) => r.id === routineId),
	)
	const tasks = useDemoState((s) => s.tasks)
	const notice = useRoutineNotice()
	const now = useNow()
	const active = resolveTab(tab)

	if (!routine) {
		return (
			<Page>
				<DemoBanner />
				<EmptyState
					icon={Repeat}
					title="Routine introuvable"
					description="Cette routine n'existe pas ou a été perdue au rechargement."
					action={
						<Button asChild variant="outline">
							<Link to="/routines">Retour aux routines</Link>
						</Button>
					}
				/>
			</Page>
		)
	}

	const label = scheduleLabel(routine)
	const upcoming = upcomingRuns(routine, now, 3)

	return (
		<Page>
			<DemoBanner />
			<PageHeader
				title={
					<span className="flex items-center gap-2.5">
						<AgentAvatar name={routine.agent} size="lg" />
						{routine.name}
					</span>
				}
				description={`${shortName(routine.agent)} · ${label} · ${routine.enabled ? 'Active' : 'En pause'}`}
				actions={
					<>
						<Button
							type="button"
							variant="outline"
							onClick={() => toggleRoutine(routine.id)}
						>
							{routine.enabled ? 'Mettre en pause' : 'Activer'}
						</Button>
						<Button type="button" onClick={() => launchRoutine(routine.id)}>
							<Play aria-hidden="true" />
							Lancer maintenant
						</Button>
					</>
				}
			/>
			<Notice message={notice} />
			<TabNav
				label="Sections de la routine"
				tabs={ROUTINE_TABS.map((t) => ({
					label: t.label,
					link: {
						to: '/routines/$routineId/{-$tab}',
						params: { routineId: routine.id, tab: t.key },
					},
				}))}
			/>
			{active === 'apercu' ? (
				<OverviewTab
					routine={routine}
					scheduleLabel={label}
					upcoming={upcoming}
				/>
			) : null}
			{active === 'declencheurs' ? (
				<TriggersPanel routine={routine} now={now} />
			) : null}
			{active === 'executions' ? (
				<RunsTab
					runs={routine.runs}
					knownTaskIds={new Set(tasks.map((t) => t.id))}
				/>
			) : null}
			{active === 'reglages' ? <SettingsTab routine={routine} /> : null}
		</Page>
	)
}
