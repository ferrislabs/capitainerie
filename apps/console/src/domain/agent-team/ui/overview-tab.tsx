import { Link } from '@tanstack/react-router'
import { SectionCard } from '#/components/layout/section-card'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import type { AgentForm } from '#/domain/agent-team/agent-config'
import { hueOf } from '#/domain/agent-team/agent-identity'
import { FieldRow } from '#/domain/agent-team/ui/field-row'
import type { DemoAgent, DemoTask } from '#/domain/demo'

interface Props {
	agent: DemoAgent
	form: AgentForm
	skills: ReadonlyArray<string>
	paused: boolean
	subCount: number
	tasks: ReadonlyArray<DemoTask>
	onCopyKey: () => void
}

export function OverviewTab({
	agent,
	form,
	skills,
	paused,
	subCount,
	tasks,
	onCopyKey,
}: Props) {
	const params = { agentId: agent.id }
	const status = paused
		? 'en pause'
		: agent.status === 'running'
			? 'en cours'
			: 'inactif'
	const variant = paused
		? 'attention'
		: agent.status === 'running'
			? 'running'
			: 'success'
	return (
		<div className="grid gap-4 lg:grid-cols-2">
			<div className="flex min-w-0 flex-col gap-4">
				<SectionCard title="Identité" flush>
					<FieldRow label="Statut">
						<Badge variant={variant}>{status}</Badge>
					</FieldRow>
					<FieldRow label="Rôle">{agent.role}</FieldRow>
					<FieldRow label="Titre">{form.title}</FieldRow>
					<FieldRow label="Rapporte à">{form.reports}</FieldRow>
					<FieldRow label="Rapports directs">{subCount}</FieldRow>
					<FieldRow label="Clé publique">
						<span className="font-mono text-xs">
							sha256:{(hueOf(agent.name) * 7919).toString(16).padStart(4, '0')}
							Kf3y…
						</span>{' '}
						<Button type="button" variant="link" size="sm" onClick={onCopyKey}>
							Copier
						</Button>
					</FieldRow>
				</SectionCard>
				<SectionCard
					title="Harness / Runtime"
					flush
					actions={
						<Link
							to="/agents/equipe/$agentId/{-$tab}"
							params={{ ...params, tab: 'runtime' }}
							className="text-link hover:underline"
						>
							Configurer
						</Link>
					}
				>
					<FieldRow label="Adaptateur">{form.adapter}</FieldRow>
					<FieldRow label="Modèle">{agent.model}</FieldRow>
					<FieldRow label="Session">
						<span className="font-mono text-xs">b2ce1171-4468-4084</span>
					</FieldRow>
					<FieldRow label="Dernier run">scheduled_retry · il y a 4 j</FieldRow>
				</SectionCard>
				<SectionCard
					title="Compétences"
					actions={
						<Link
							to="/agents/equipe/$agentId/{-$tab}"
							params={{ ...params, tab: 'skills' }}
							className="text-link hover:underline"
						>
							Gérer
						</Link>
					}
				>
					<ul className="flex flex-wrap gap-1.5">
						{skills.map((skill) => (
							<li key={skill}>
								<Badge variant="secondary">{skill}</Badge>
							</li>
						))}
					</ul>
				</SectionCard>
			</div>
			<SectionCard title="Tâches récentes" flush>
				{tasks.length === 0 ? (
					<p className="px-3 py-4 text-muted-foreground">
						Aucune tâche pour le moment.
					</p>
				) : (
					<ul className="divide-y">
						{tasks.map((task) => (
							<li key={task.id} className="flex items-center gap-2 px-3 py-2">
								<Badge variant="outline">{task.id}</Badge>
								<span className="min-w-0 flex-1 truncate">{task.title}</span>
								<span className="text-xs text-muted-foreground">
									{task.age}
								</span>
							</li>
						))}
					</ul>
				)}
			</SectionCard>
		</div>
	)
}
