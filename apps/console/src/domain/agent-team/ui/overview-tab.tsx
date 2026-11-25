import { Link } from '@tanstack/react-router'
import { ArrowRight } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import type { AgentForm } from '#/domain/agent-team/agent-config'
import type { TeamAgent } from '#/domain/agent-team/team-agent'
import { InfoList, InfoRow } from '#/domain/agent-team/ui/field-row'
import { IdentitySection } from '#/domain/agent-team/ui/identity-section'
import { DetailSection, SectionGrid } from '#/domain/agent-team/ui/panel'
import { Chip, ChipList, Empty } from '#/domain/agent-team/ui/rows'
import { Sev, type SevTone } from '#/domain/agent-team/ui/sev'
import { relativeAge } from '#/domain/dashboard/format'
import type { Task } from '#/domain/tasks/model'
import { shortTaskId, taskPathId } from '#/domain/tasks/task-ids'
import { StatusIcon } from '#/domain/tasks/ui/status-icon'

interface Props {
	agent: TeamAgent
	form: AgentForm
	adapter: string
	subCount: number
	reportsTo: string
	tasks: ReadonlyArray<Task>
	taskCount: number
	tasksLoading: boolean
	tasksError: boolean
	activeRuns: ReadonlyArray<Schemas.ActiveRunResponse>
	identity: Parameters<typeof IdentitySection>[0]
}

const RUN_STATUS: Record<
	Schemas.RunStatusResponse,
	{ label: string; tone: SevTone }
> = {
	queued: { label: 'en attente', tone: 'info' },
	running: { label: 'en cours', tone: 'vio' },
	succeeded: { label: 'réussie', tone: 'ok' },
	failed: { label: 'échouée', tone: 'crit' },
	cancelled: { label: 'annulée', tone: 'info' },
}

const runTitle = (run: Schemas.ActiveRunResponse) =>
	run.run.title ?? run.run.transcript_title ?? 'Run'

const runStart = (run: Schemas.ActiveRunResponse) =>
	run.run.started_at ?? run.run.created_at

const stepOf = (run: Schemas.ActiveRunResponse) =>
	Math.min(run.steps_done + 1, run.steps_total)

function RunSummary({ run }: { run: Schemas.ActiveRunResponse | undefined }) {
	if (!run) return <>Aucun run en cours</>
	const label = `${runTitle(run)} · étape ${stepOf(run)}/${run.steps_total} · ${relativeAge(runStart(run))}`
	if (!run.run.card_id) return <>{label}</>
	return (
		<Link
			to="/tasks/$taskId"
			params={{ taskId: taskPathId(run.run.card_id) }}
			className="text-primary hover:underline"
		>
			{label}
		</Link>
	)
}

function LatestRun({ run }: { run: Schemas.ActiveRunResponse }) {
	const status = RUN_STATUS[run.run.status]
	const step = run.run.status === 'running' ? run.current_step : undefined
	const cardId = run.run.card_id
	return (
		<DetailSection title="Dernière exécution">
			<div className="flex flex-col gap-3">
				<div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2">
					<Sev tone={status.tone} className="px-2.5 py-0.5 text-xs">
						{status.label}
					</Sev>
					{cardId ? (
						<Link
							to="/tasks/$taskId"
							params={{ taskId: taskPathId(cardId) }}
							className="font-mono text-[13px] text-muted-foreground hover:text-foreground"
						>
							{shortTaskId(cardId)}
						</Link>
					) : null}
					<span className="min-w-0 flex-1 truncate text-[15px] font-medium">
						{runTitle(run)}
					</span>
					<span className="whitespace-nowrap text-muted-foreground">
						{relativeAge(runStart(run))}
					</span>
				</div>
				<p className="m-0 text-[15px] text-muted-foreground">
					Étape {stepOf(run)} sur {run.steps_total}
					{step ? ` · ${step.agent ?? step.kind}` : ''}
					{run.run.pipeline.length > 0
						? ` · chaîne ${run.run.pipeline.join(' → ')}`
						: ''}
				</p>
			</div>
		</DetailSection>
	)
}

function TaskList({
	tasks,
	total,
	loading,
	error,
}: {
	tasks: ReadonlyArray<Task>
	total: number
	loading: boolean
	error: boolean
}) {
	if (error)
		return (
			<p role="alert" className="m-0 px-6 py-5 text-signal-danger">
				Impossible de charger les tâches.
			</p>
		)
	if (loading)
		return (
			<output className="m-0 block px-6 py-5 text-muted-foreground">
				Chargement des tâches…
			</output>
		)
	if (tasks.length === 0)
		return <Empty>Aucune tâche assignée à cet agent.</Empty>
	const more = total - tasks.length
	return (
		<>
			<ul className="m-0 list-none p-0">
				{tasks.map((task) => (
					<li key={task.id} className="border-t first:border-t-0">
						<Link
							to="/tasks/$taskId"
							params={{ taskId: taskPathId(task.id) }}
							className="flex min-h-14 min-w-0 items-center gap-4 px-6 py-3 text-[15px] hover:bg-secondary/60 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring"
						>
							<StatusIcon status={task.status} className="size-[18px]" />
							<span className="min-w-0 flex-1 truncate">{task.title}</span>
							<span className="whitespace-nowrap text-[13px] text-muted-foreground">
								{task.age}
							</span>
							<span className="hidden w-20 text-right font-mono text-[13px] text-muted-foreground sm:inline">
								{task.shortId}
							</span>
						</Link>
					</li>
				))}
			</ul>
			{more > 0 ? (
				<p className="m-0 border-t px-6 py-3.5 text-center text-[13px] text-muted-foreground">
					+{more} {more > 1 ? 'autres tâches' : 'autre tâche'}
				</p>
			) : null}
		</>
	)
}

export function OverviewTab({
	agent,
	form,
	adapter,
	subCount,
	reportsTo,
	tasks,
	taskCount,
	tasksLoading,
	tasksError,
	activeRuns,
	identity,
}: Props) {
	const params = { agentId: agent.id }
	const status = agent.paused
		? 'en pause'
		: agent.status === 'running'
			? 'en cours'
			: 'inactif'
	const tone = agent.paused ? 'warn' : agent.status === 'running' ? 'vio' : 'ok'
	const latest = activeRuns[0]
	const access = agent.connector_grants.length + agent.repository_grants.length
	return (
		<>
			{latest ? <LatestRun run={latest} /> : null}

			<SectionGrid>
				<DetailSection
					title="Identité"
					actions={
						<Sev tone={tone} className="rounded-full px-2.5 py-0.5 text-xs">
							{status}
						</Sev>
					}
				>
					<InfoList>
						<InfoRow label="Rôle">{agent.role}</InfoRow>
						<InfoRow label="Description">{agent.description || '—'}</InfoRow>
						<InfoRow label="Titre">{form.title || '—'}</InfoRow>
						<InfoRow label="Rapporte à">{reportsTo}</InfoRow>
						<InfoRow label="Rapports directs">{subCount}</InfoRow>
						<InfoRow label="Révision">{agent.revision}</InfoRow>
					</InfoList>
				</DetailSection>
				<DetailSection
					title="Harness / Runtime"
					actions={
						<Link
							to="/agents/$agentId/{-$tab}"
							params={{ ...params, tab: 'runtime' }}
						>
							Configurer
						</Link>
					}
				>
					<InfoList>
						<InfoRow label="Adaptateur">{adapter}</InfoRow>
						<InfoRow label="Modèle" mono={Boolean(agent.model)}>
							{agent.model || 'Modèle par défaut du fournisseur'}
						</InfoRow>
						<InfoRow label="Run en cours">
							<RunSummary run={latest} />
						</InfoRow>
					</InfoList>
				</DetailSection>

				<DetailSection
					title="Compétences"
					actions={
						<Link
							to="/agents/$agentId/{-$tab}"
							params={{ ...params, tab: 'skills' }}
						>
							Gérer
						</Link>
					}
				>
					{agent.skills.length === 0 ? (
						<p className="m-0 text-muted-foreground">
							Aucune compétence activée.
						</p>
					) : (
						<ChipList>
							{agent.skills.map((skill) => (
								<Chip key={skill}>{skill}</Chip>
							))}
						</ChipList>
					)}
				</DetailSection>
				<DetailSection
					title="Connecteurs et dépôts"
					actions={
						<Link
							to="/agents/$agentId/{-$tab}"
							params={{ ...params, tab: 'connectors' }}
						>
							Gérer
						</Link>
					}
				>
					{access === 0 ? (
						<p className="m-0 text-muted-foreground">
							Aucun connecteur ni dépôt autorisé.
						</p>
					) : (
						<div className="flex flex-col gap-5">
							{agent.connector_grants.length > 0 ? (
								<ChipList>
									{agent.connector_grants.map((slug) => (
										<Chip key={slug}>{slug}</Chip>
									))}
								</ChipList>
							) : null}
							{agent.repository_grants.length > 0 ? (
								<ul className="m-0 flex list-none flex-col gap-2 p-0 font-mono text-[13.5px]">
									{agent.repository_grants.map((name) => (
										<li key={name} className="truncate">
											{name}
										</li>
									))}
								</ul>
							) : null}
						</div>
					)}
				</DetailSection>
			</SectionGrid>

			<IdentitySection {...identity} />

			<DetailSection
				title="Tâches récentes"
				flush
				actions={
					<Link
						to="/tasks"
						search={{ vue: 'liste', q: '' }}
						className="inline-flex items-center gap-1"
					>
						Tout voir
						<ArrowRight aria-hidden="true" className="size-3.5" />
					</Link>
				}
			>
				<TaskList
					tasks={tasks}
					total={taskCount}
					loading={tasksLoading}
					error={tasksError}
				/>
			</DetailSection>
		</>
	)
}
