import { Link } from '@tanstack/react-router'
import type { Schemas } from '#/api/api.client'
import { SectionCard } from '#/components/layout/section-card'
import type { AgentForm } from '#/domain/agent-team/agent-config'
import type { TeamAgent } from '#/domain/agent-team/team-agent'
import { FieldRow } from '#/domain/agent-team/ui/field-row'
import { Chip, ChipList, Empty, Rows } from '#/domain/agent-team/ui/rows'
import { Sev } from '#/domain/agent-team/ui/sev'
import { TaskLine } from '#/domain/agent-team/ui/task-line'
import { relativeAge } from '#/domain/dashboard/format'
import type { Task } from '#/domain/tasks/model'
import { taskPathId } from '#/domain/tasks/task-ids'

interface Props {
	agent: TeamAgent
	form: AgentForm
	subCount: number
	reportsTo: string
	tasks: ReadonlyArray<Task>
	tasksLoading: boolean
	tasksError: boolean
	activeRuns: ReadonlyArray<Schemas.ActiveRunResponse>
}

function RunSummary({ run }: { run: Schemas.ActiveRunResponse | undefined }) {
	if (!run) return <>Aucun run en cours</>
	const title = run.run.title ?? run.run.transcript_title ?? 'Run'
	const started = run.run.started_at ?? run.run.created_at
	const label = `${title} · étape ${Math.min(run.steps_done + 1, run.steps_total)}/${run.steps_total} · ${relativeAge(started)}`
	if (!run.run.card_id) return <>{label}</>
	return (
		<Link
			to="/tasks/$taskId"
			params={{ taskId: taskPathId(run.run.card_id) }}
			className="text-primary"
		>
			{label}
		</Link>
	)
}

export function OverviewTab({
	agent,
	form,
	subCount,
	reportsTo,
	tasks,
	tasksLoading,
	tasksError,
	activeRuns,
}: Props) {
	const params = { agentId: agent.id }
	const status = agent.paused
		? 'en pause'
		: agent.status === 'running'
			? 'en cours'
			: 'inactif'
	const tone = agent.paused ? 'warn' : agent.status === 'running' ? 'vio' : 'ok'
	return (
		<div className="grid grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)] items-start gap-2.5">
			<div>
				<SectionCard title="Identité" flush>
					<FieldRow label="Statut">
						<Sev tone={tone}>{status}</Sev>
					</FieldRow>
					<FieldRow label="Rôle">{agent.role}</FieldRow>
					<FieldRow label="Description">{agent.description || '—'}</FieldRow>
					<FieldRow label="Titre">{form.title || '—'}</FieldRow>
					<FieldRow label="Rapporte à">{reportsTo}</FieldRow>
					<FieldRow label="Rapports directs">{subCount}</FieldRow>
					<FieldRow label="Révision">{agent.revision}</FieldRow>
				</SectionCard>
				<SectionCard
					title="Harness / Runtime"
					flush
					actions={
						<Link
							to="/agents/$agentId/{-$tab}"
							params={{ ...params, tab: 'runtime' }}
						>
							Configurer
						</Link>
					}
				>
					<FieldRow label="Adaptateur">{form.adapter}</FieldRow>
					<FieldRow label="Modèle">
						{agent.model || 'Modèle par défaut du fournisseur'}
					</FieldRow>
					<FieldRow label="Run en cours">
						<RunSummary run={activeRuns[0]} />
					</FieldRow>
				</SectionCard>
				<SectionCard
					title="Compétences"
					flush
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
						<Empty className="p-3.5">Aucune compétence activée.</Empty>
					) : (
						<ChipList>
							{agent.skills.map((skill) => (
								<Chip key={skill}>{skill}</Chip>
							))}
						</ChipList>
					)}
				</SectionCard>
			</div>
			<div>
				<SectionCard
					title="Tâches récentes"
					flush
					actions={
						<Link to="/tasks" search={{ vue: 'liste', q: '' }}>
							Tout voir
						</Link>
					}
				>
					{tasksError ? (
						<p role="alert" className="m-0 p-3 text-signal-danger">
							Impossible de charger les tâches.
						</p>
					) : tasksLoading ? (
						<output className="m-0 block p-3 text-muted-foreground">
							Chargement des tâches…
						</output>
					) : tasks.length === 0 ? (
						<Empty>Aucune tâche assignée à cet agent.</Empty>
					) : (
						<Rows>
							{tasks.map((task) => (
								<TaskLine key={task.id} task={task} assignee={agent} />
							))}
						</Rows>
					)}
				</SectionCard>
			</div>
		</div>
	)
}
