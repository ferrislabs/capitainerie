import { Link } from '@tanstack/react-router'
import { useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import type { AgentForm } from '#/domain/agent-team/agent-config'
import type { TeamAgent } from '#/domain/agent-team/team-agent'
import { AgentAvatar } from '#/domain/agent-team/ui/agent-avatar'
import { CTL, FieldRow, Note } from '#/domain/agent-team/ui/field-row'
import { Options } from '#/domain/agent-team/ui/options'
import { Empty, ROW, Rows, Ttl } from '#/domain/agent-team/ui/rows'
import { TaskLine } from '#/domain/agent-team/ui/task-line'
import { Toggle } from '#/domain/agent-team/ui/toggle'
import type { DemoTask } from '#/domain/demo'

export interface SubAgentEntry {
	agent: TeamAgent
	openTasks: number
}

export interface DelegatedTask {
	task: DemoTask
	assignee: TeamAgent | undefined
}

interface Props {
	agent: TeamAgent
	ancestors: ReadonlyArray<TeamAgent>
	subs: ReadonlyArray<SubAgentEntry>
	candidates: ReadonlyArray<TeamAgent>
	recent: ReadonlyArray<DelegatedTask>
	form: AgentForm
	onChange: (patch: Partial<AgentForm>) => void
	onRemove: (subId: string) => void
	onAdd: (subId: string) => void
	onAssign: (sub: TeamAgent) => void
}

function Choice({
	label,
	hint,
	value,
	values,
	onChange,
}: {
	label: string
	hint?: string
	value: string
	values: ReadonlyArray<string>
	onChange: (value: string) => void
}) {
	return (
		<FieldRow label={label} hint={hint}>
			<select
				aria-label={label}
				className={CTL}
				value={value}
				onChange={(event) => onChange(event.target.value)}
			>
				<Options values={values} />
			</select>
		</FieldRow>
	)
}

export function DelegationTab({
	agent,
	ancestors,
	subs,
	candidates,
	recent,
	form,
	onChange,
	onRemove,
	onAdd,
	onAssign,
}: Props) {
	const [chosen, setChosen] = useState<string>()
	const selected = candidates.some((c) => c.id === chosen)
		? chosen
		: candidates[0]?.id
	const parent = ancestors[0]
	return (
		<>
			<SectionCard
				title="Place dans l'organisation"
				flush
				actions={<Link to="/agents/organigramme">Organigramme complet</Link>}
			>
				<Rows>
					{parent ? (
						<Link
							to="/agents/$agentId/{-$tab}"
							params={{ agentId: parent.id }}
							className={ROW}
						>
							<AgentAvatar hue={parent.hue} size={24} />
							<Ttl title={parent.name} sub="Rapporte à lui · agent parent" />
						</Link>
					) : (
						<div className={ROW}>
							<Ttl title="Aucun agent parent" sub="Direction" />
						</div>
					)}
					<div className={ROW} style={{ paddingLeft: 34 }}>
						<span aria-hidden="true" className="text-faint">
							↳
						</span>
						<AgentAvatar hue={agent.hue} size={24} />
						<Ttl title={agent.name} sub="Cet agent" />
					</div>
					{subs.map(({ agent: sub }) => (
						<Link
							key={sub.id}
							to="/agents/$agentId/{-$tab}"
							params={{ agentId: sub.id }}
							className={ROW}
							style={{ paddingLeft: 56 }}
						>
							<span aria-hidden="true" className="text-faint">
								↳
							</span>
							<AgentAvatar hue={sub.hue} size={22} />
							<Ttl title={sub.name} sub={sub.role} />
						</Link>
					))}
				</Rows>
			</SectionCard>

			<SectionCard
				title="Sous-agents"
				flush
				actions={<Link to="/agents/nouveau">Créer un sous-agent</Link>}
			>
				{subs.length === 0 ? (
					<Empty title="Aucun sous-agent">
						Cet agent fait tout lui-même. Ajoute-en un pour lui permettre de
						déléguer.
					</Empty>
				) : (
					<Rows>
						{subs.map(({ agent: sub, openTasks }) => (
							<div key={sub.id} className={ROW}>
								<AgentAvatar hue={sub.hue} size={28} />
								<Ttl
									title={
										<Link
											to="/agents/$agentId/{-$tab}"
											params={{ agentId: sub.id }}
											className="text-foreground"
										>
											{sub.name}
										</Link>
									}
									sub={sub.description}
								/>
								<span className="whitespace-nowrap text-muted-foreground tabular-nums">
									{openTasks} {openTasks > 1 ? 'ouvertes' : 'ouverte'}
								</span>
								<Button
									type="button"
									variant="outline"
									size="sm"
									aria-label={`Assigner ${sub.name}`}
									onClick={() => onAssign(sub)}
								>
									Assigner
								</Button>
								<Button
									type="button"
									variant="outline"
									size="sm"
									aria-label={`Retirer ${sub.name}`}
									onClick={() => onRemove(sub.id)}
								>
									Retirer
								</Button>
							</div>
						))}
					</Rows>
				)}
			</SectionCard>

			{candidates.length > 0 ? (
				<SectionCard title="Ajouter un sous-agent" flush>
					<div className="flex gap-1.5 px-3 pt-3 pb-3">
						<select
							aria-label="Agent à ajouter"
							className={CTL}
							value={selected}
							onChange={(event) => setChosen(event.target.value)}
						>
							{candidates.map((c) => (
								<option key={c.id} value={c.id}>
									{c.name}
								</option>
							))}
						</select>
						<Button
							type="button"
							disabled={!selected}
							onClick={() => selected && onAdd(selected)}
						>
							Ajouter
						</Button>
					</div>
					<Note>
						Il rapportera à cet agent à la place de son parent actuel.
					</Note>
				</SectionCard>
			) : null}

			<SectionCard title="Règles de délégation" flush>
				<FieldRow
					label="Peut déléguer"
					hint="Crée des sous-tâches pour ses sous-agents"
				>
					<Toggle
						label="Peut déléguer"
						checked={form.canDeleg}
						onChange={(canDeleg) => onChange({ canDeleg })}
					/>
				</FieldRow>
				<Choice
					label="Délègue à"
					value={form.dScope}
					values={[
						'Équipe directe',
						'Équipe et sous-équipes',
						"Toute l'organisation",
					]}
					onChange={(dScope) => onChange({ dScope })}
				/>
				<Choice
					label="Profondeur maximale"
					hint="Un sous-agent peut-il déléguer à son tour"
					value={form.dDepth}
					values={['1', '2', '3']}
					onChange={(dDepth) => onChange({ dDepth })}
				/>
				<Choice
					label="Sous-tâches en parallèle"
					value={form.dPar}
					values={['1', '2', '3', '5', '10']}
					onChange={(dPar) => onChange({ dPar })}
				/>
				<Choice
					label="Budget par sous-tâche"
					hint="Plafond de dépense API"
					value={form.dBudget}
					values={['1 $', '5 $', '10 $', 'Illimité']}
					onChange={(dBudget) => onChange({ dBudget })}
				/>
				<Choice
					label="Résultats relus par"
					value={form.dReview}
					values={['Aucun (automatique)', "L'agent parent", 'Une personne']}
					onChange={(dReview) => onChange({ dReview })}
				/>
				<Choice
					label="Si un sous-agent échoue"
					value={form.dFail}
					values={[
						'Réessayer 2 fois',
						"Remonter à l'agent parent",
						'Bloquer la tâche parente',
					]}
					onChange={(dFail) => onChange({ dFail })}
				/>
				<Note>
					La tâche parente reste ouverte tant que ses sous-tâches le sont. Un
					sous-agent n'hérite d'aucun connecteur de son parent : chaque accès
					s'autorise séparément.
				</Note>
			</SectionCard>

			<SectionCard title="Sous-tâches déléguées récemment" flush>
				{recent.length === 0 ? (
					<Empty title="Aucune délégation" />
				) : (
					<Rows>
						{recent.map(({ task, assignee }) => (
							<TaskLine
								key={task.id}
								task={task}
								assignee={assignee}
								sub={`Parent ${task.parentId}`}
								showName={false}
							/>
						))}
					</Rows>
				)}
			</SectionCard>
		</>
	)
}
