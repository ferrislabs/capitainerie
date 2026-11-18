import { Link, useNavigate } from '@tanstack/react-router'
import { Play, Repeat } from 'lucide-react'
import { useState } from 'react'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Notice, useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { SectionCard } from '#/components/layout/section-card'
import { TabNav } from '#/components/layout/tab-nav'
import { Button } from '#/components/ui/button'
import { RunsPanel } from '#/domain/routines/feature/runs-panel'
import { TriggersPanel } from '#/domain/routines/feature/triggers-panel'
import {
	useNow,
	useRoutineFormOptions,
} from '#/domain/routines/feature/use-routine-context'
import {
	useArchiveRoutine,
	useLaunchRoutine,
	useRoutine,
	useUpdateRoutine,
} from '#/domain/routines/hooks/use-routines'
import {
	describeRoutineError,
	draftOf,
	type Routine,
	type RoutineDraft,
	triggersSummary,
	updateBody,
} from '#/domain/routines/model'
import { OverviewTab } from '#/domain/routines/ui/overview-tab'
import { RoutineForm } from '#/domain/routines/ui/routine-form'
import { AgentAvatar } from '#/domain/sources/ui/agent-avatar'
import { Sev } from '#/domain/sources/ui/kit'

export const ROUTINE_TABS = [
	{ key: 'apercu', label: "Vue d'ensemble" },
	{ key: 'declencheurs', label: 'Déclencheurs' },
	{ key: 'executions', label: 'Passages' },
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
	const query = useRoutine(routineId)
	if (query.isPending) {
		return (
			<Page>
				<output className="text-muted-foreground">
					Chargement de la routine…
				</output>
			</Page>
		)
	}
	if (query.isError) {
		const missing = (query.error as { status?: number } | null)?.status === 404
		return (
			<Page>
				{missing ? (
					<EmptyState
						icon={Repeat}
						title="Routine introuvable"
						description="Cette routine n'existe pas ou a été supprimée."
						action={
							<Button asChild variant="outline">
								<Link to="/routines">Retour aux routines</Link>
							</Button>
						}
					/>
				) : (
					<div className="grid gap-2">
						<InlineAlert tone="danger">
							Impossible de charger la routine.
						</InlineAlert>
						<div>
							<Button variant="outline" onClick={() => void query.refetch()}>
								Réessayer
							</Button>
						</div>
					</div>
				)}
			</Page>
		)
	}
	return <RoutineDetail key={query.data.id} routine={query.data} tab={tab} />
}

function RoutineDetail({
	routine,
	tab,
}: {
	routine: Routine
	tab?: string | undefined
}) {
	const navigate = useNavigate()
	const [message, notify] = useNotice()
	const [formError, setFormError] = useState<string | null>(null)
	const now = useNow()
	const { agents, projects } = useRoutineFormOptions()
	const update = useUpdateRoutine()
	const archive = useArchiveRoutine()
	const launch = useLaunchRoutine()
	const current = resolveTab(tab)
	const path = { routine_id: routine.id }
	const fail = (error: unknown) => notify(describeRoutineError(error))
	const agent = agents.find((a) => a.id === routine.agent_id)
	const projectName = projects.find((p) => p.id === routine.project_id)?.name

	const tabs = ROUTINE_TABS.map((t) => ({
		label: t.label,
		link: {
			to: '/routines/$routineId/{-$tab}' as const,
			params:
				t.key === 'apercu' && current === 'apercu'
					? { routineId: routine.id, tab }
					: { routineId: routine.id, tab: t.key },
		},
	}))

	const setEnabled = (enabled: boolean) =>
		update.mutate(
			{ path, body: { enabled } },
			{
				onSuccess: () =>
					notify(
						`${enabled ? 'Routine activée' : 'Routine en pause'} : ${routine.name}`,
					),
				onError: fail,
			},
		)

	const runNow = () =>
		launch.mutate(
			{ path },
			{
				onSuccess: () => {
					notify('Routine lancée : la tâche est en cours de création.')
					if (current !== 'executions')
						void navigate({
							to: '/routines/$routineId/{-$tab}',
							params: { routineId: routine.id, tab: 'executions' },
						})
				},
				onError: fail,
			},
		)

	const save = (draft: RoutineDraft) => {
		setFormError(null)
		const body = updateBody(routine, draft)
		if (Object.keys(body).length === 0) {
			notify('Aucune modification')
			return
		}
		update.mutate(
			{ path, body },
			{
				onSuccess: () => notify('Routine enregistrée'),
				onError: (error) => setFormError(describeRoutineError(error)),
			},
		)
	}

	const setArchived = (archived: boolean) => {
		const done = {
			onSuccess: () =>
				notify(archived ? 'Routine archivée' : 'Routine restaurée'),
			onError: fail,
		}
		if (archived) archive.mutate({ path }, done)
		else update.mutate({ path, body: { archived: false } }, done)
	}

	const state = routine.archived
		? 'Archivée'
		: routine.enabled
			? 'Active'
			: 'En pause'

	return (
		<Page>
			<header className="mb-3 flex flex-wrap items-end gap-x-3.5 gap-y-2.5">
				{agent ? <AgentAvatar agent={agent} size={36} decorative /> : null}
				<div className="min-w-0">
					<h1 className="m-0 text-xl leading-[1.2] font-semibold tracking-[-0.01em]">
						{routine.name}
					</h1>
					<span className="flex flex-wrap items-center gap-1.5 text-muted-foreground">
						<Sev
							tone={routine.archived ? 'info' : routine.enabled ? 'ok' : 'warn'}
						>
							{state}
						</Sev>
						{`${agent?.name ?? 'Agent inconnu'} · ${triggersSummary(routine)}`}
					</span>
				</div>
				{routine.archived ? null : (
					<div className="ml-auto flex flex-wrap items-center gap-2">
						<Button
							type="button"
							variant="outline"
							disabled={update.isPending}
							onClick={() => setEnabled(!routine.enabled)}
						>
							{routine.enabled ? 'Mettre en pause' : 'Activer'}
						</Button>
						<Button type="button" disabled={launch.isPending} onClick={runNow}>
							<Play aria-hidden="true" />
							Lancer maintenant
						</Button>
					</div>
				)}
			</header>
			<Notice message={message} className="mb-3" />
			<TabNav label="Sections de la routine" tabs={tabs} />
			{current === 'apercu' ? (
				<OverviewTab
					routine={routine}
					agentName={agent?.name}
					projectName={projectName}
					now={now}
				/>
			) : null}
			{current === 'declencheurs' ? (
				<TriggersPanel routine={routine} now={now} notify={notify} />
			) : null}
			{current === 'executions' ? (
				<RunsPanel routineId={routine.id} now={now} />
			) : null}
			{current === 'reglages' ? (
				<div className="grid gap-4">
					{routine.archived ? null : (
						<RoutineForm
							mode="edit"
							initial={draftOf(routine)}
							agents={agents}
							projects={projects}
							pending={update.isPending}
							error={formError}
							onSubmit={save}
						/>
					)}
					<SectionCard
						title={routine.archived ? 'Routine archivée' : 'Archiver'}
						className="max-w-[880px]"
					>
						<div className="flex flex-wrap items-center gap-3">
							<p className="m-0 min-w-0 flex-1 text-muted-foreground">
								{routine.archived
									? 'Restaurer la routine la remet en service avec ses déclencheurs.'
									: "Plus aucun déclencheur ne crée de tâche ; l'historique des passages reste consultable."}
							</p>
							<Button
								type="button"
								variant="outline"
								disabled={archive.isPending || update.isPending}
								onClick={() => setArchived(!routine.archived)}
							>
								{routine.archived ? 'Restaurer' : 'Archiver la routine'}
							</Button>
						</div>
					</SectionCard>
				</div>
			) : null}
		</Page>
	)
}
