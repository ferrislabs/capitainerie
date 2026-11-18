import { Link } from '@tanstack/react-router'
import { SectionCard } from '#/components/layout/section-card'
import type { AgentForm } from '#/domain/agent-team/agent-config'
import { hueOf } from '#/domain/agent-team/agent-identity'
import type { TeamAgent } from '#/domain/agent-team/team-agent'
import { FieldRow } from '#/domain/agent-team/ui/field-row'
import { Chip, ChipList, Empty, Rows } from '#/domain/agent-team/ui/rows'
import { Sev } from '#/domain/agent-team/ui/sev'
import { TaskLine } from '#/domain/agent-team/ui/task-line'
import type { DemoTask } from '#/domain/demo'

interface Props {
	agent: TeamAgent
	form: AgentForm
	skills: ReadonlyArray<string>
	paused: boolean
	subCount: number
	reportsTo: string
	tasks: ReadonlyArray<DemoTask>
	onCopyKey: () => void
}

export function OverviewTab({
	agent,
	form,
	skills,
	paused,
	subCount,
	reportsTo,
	tasks,
	onCopyKey,
}: Props) {
	const params = { agentId: agent.id }
	const status = paused
		? 'en pause'
		: agent.status === 'running'
			? 'en cours'
			: 'inactif'
	const tone = paused ? 'warn' : agent.status === 'running' ? 'vio' : 'ok'
	return (
		<div className="grid grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)] items-start gap-2.5">
			<div>
				<SectionCard title="Identité" flush>
					<FieldRow label="Statut">
						<Sev tone={tone}>{status}</Sev>
					</FieldRow>
					<FieldRow label="Rôle">{agent.role}</FieldRow>
					<FieldRow label="Description">{agent.description || '—'}</FieldRow>
					<FieldRow label="Titre">{form.title}</FieldRow>
					<FieldRow label="Rapporte à">{reportsTo}</FieldRow>
					<FieldRow label="Rapports directs">{subCount}</FieldRow>
					<FieldRow label="Clé publique">
						<span className="font-mono text-[11.5px]">
							sha256:{(hueOf(agent.name) * 7919).toString(16).padStart(4, '0')}
							Kf3y…
						</span>{' '}
						<button
							type="button"
							className="text-xs text-primary"
							onClick={onCopyKey}
						>
							Copier
						</button>
					</FieldRow>
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
					<FieldRow label="Modèle">{agent.model}</FieldRow>
					<FieldRow label="Session">
						<span className="font-mono text-[11.5px]">b2ce1171-4468-4084</span>
					</FieldRow>
					<FieldRow label="Dernier run">scheduled_retry · il y a 4 j</FieldRow>
				</SectionCard>
				<SectionCard title="Capacités" flush>
					<Empty className="p-3.5">
						Aucun résumé de capacités. Ajoute-le dans les instructions.
					</Empty>
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
					<ChipList>
						{skills.map((skill) => (
							<Chip key={skill}>{skill}</Chip>
						))}
					</ChipList>
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
					{tasks.length === 0 ? (
						<Empty>Aucune tâche pour le moment.</Empty>
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
