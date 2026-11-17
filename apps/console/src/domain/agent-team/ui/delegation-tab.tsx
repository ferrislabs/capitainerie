import { Link } from '@tanstack/react-router'
import { useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import { Switch } from '#/components/ui/switch'
import type { AgentForm } from '#/domain/agent-team/agent-config'
import { AgentAvatar } from '#/domain/agent-team/ui/agent-avatar'
import { FieldRow } from '#/domain/agent-team/ui/field-row'
import { Options } from '#/domain/agent-team/ui/options'
import type { DemoAgent, DemoTask } from '#/domain/demo'
import { taskPathId } from '#/domain/tasks/task-ids'

export interface SubAgentEntry {
	agent: DemoAgent
	openTasks: number
}

export interface DelegatedTask {
	task: DemoTask
	assignee: DemoAgent | undefined
}

interface Props {
	agent: DemoAgent
	ancestors: ReadonlyArray<DemoAgent>
	subs: ReadonlyArray<SubAgentEntry>
	candidates: ReadonlyArray<DemoAgent>
	recent: ReadonlyArray<DelegatedTask>
	form: AgentForm
	onChange: (patch: Partial<AgentForm>) => void
	onRemove: (subId: string) => void
	onAdd: (subId: string) => void
}

function AgentLink({ agent, size }: { agent: DemoAgent; size: number }) {
	return (
		<Link
			to="/agents/equipe/$agentId/{-$tab}"
			params={{ agentId: agent.id }}
			className="flex min-w-0 items-center gap-2 hover:underline"
		>
			<AgentAvatar name={agent.name} hue={agent.hue} size={size} />
			<span className="truncate font-medium">{agent.name}</span>
		</Link>
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
}: Props) {
	const [chosen, setChosen] = useState<string>()
	const selected = candidates.some((c) => c.id === chosen)
		? chosen
		: candidates[0]?.id
	const parent = ancestors[0]
	return (
		<div className="flex flex-col gap-4">
			<SectionCard
				title="Place dans l'organisation"
				flush
				actions={
					<Link to="/agents/organigramme" className="text-link hover:underline">
						Organigramme complet
					</Link>
				}
			>
				<ul className="divide-y">
					<li className="flex items-center gap-2 px-3 py-2">
						{parent ? (
							<>
								<AgentLink agent={parent} size={24} />
								<span className="text-xs text-muted-foreground">
									Agent parent
								</span>
							</>
						) : (
							<>
								<b className="font-medium">Aucun agent parent</b>
								<span className="text-xs text-muted-foreground">Direction</span>
							</>
						)}
					</li>
					<li className="flex items-center gap-2 py-2 pr-3 pl-8">
						<span aria-hidden="true" className="text-muted-foreground">
							↳
						</span>
						<AgentAvatar name={agent.name} hue={agent.hue} size={24} />
						<b className="font-medium">{agent.name}</b>
						<span className="text-xs text-muted-foreground">Cet agent</span>
					</li>
					{subs.map(({ agent: sub }) => (
						<li
							key={sub.id}
							className="flex items-center gap-2 py-2 pr-3 pl-14"
						>
							<span aria-hidden="true" className="text-muted-foreground">
								↳
							</span>
							<AgentLink agent={sub} size={22} />
							<span className="text-xs text-muted-foreground">{sub.role}</span>
						</li>
					))}
				</ul>
			</SectionCard>

			<SectionCard title="Sous-agents" flush>
				{subs.length === 0 ? (
					<p className="px-3 py-4 text-muted-foreground">
						Aucun sous-agent. Cet agent fait tout lui-même. Ajoute-en un pour
						lui permettre de déléguer.
					</p>
				) : (
					<ul className="divide-y">
						{subs.map(({ agent: sub, openTasks }) => (
							<li key={sub.id} className="flex items-center gap-3 px-3 py-2.5">
								<span className="min-w-0 flex-1">
									<AgentLink agent={sub} size={28} />
									<span className="block truncate pl-9 text-xs text-muted-foreground">
										{sub.description}
									</span>
								</span>
								<span className="whitespace-nowrap text-muted-foreground">
									{openTasks} {openTasks > 1 ? 'ouvertes' : 'ouverte'}
								</span>
								<Button
									type="button"
									variant="outline"
									size="sm"
									aria-label={`Retirer ${sub.name}`}
									onClick={() => onRemove(sub.id)}
								>
									Retirer
								</Button>
							</li>
						))}
					</ul>
				)}
			</SectionCard>

			{candidates.length > 0 ? (
				<SectionCard
					title="Ajouter un sous-agent"
					description="Il rapportera à cet agent à la place de son parent actuel."
				>
					<div className="flex gap-2">
						<NativeSelect
							aria-label="Agent à ajouter"
							value={selected}
							onChange={(event) => setChosen(event.target.value)}
						>
							{candidates.map((c) => (
								<option key={c.id} value={c.id}>
									{c.name}
								</option>
							))}
						</NativeSelect>
						<Button
							type="button"
							disabled={!selected}
							onClick={() => selected && onAdd(selected)}
						>
							Ajouter
						</Button>
					</div>
				</SectionCard>
			) : null}

			<SectionCard title="Règles de délégation" flush>
				<FieldRow
					label="Peut déléguer"
					hint="Crée des sous-tâches pour ses sous-agents"
				>
					<Switch
						aria-label="Peut déléguer"
						checked={form.canDeleg}
						onCheckedChange={(canDeleg) => onChange({ canDeleg })}
					/>
				</FieldRow>
				<FieldRow label="Délègue à">
					<NativeSelect
						aria-label="Délègue à"
						value={form.dScope}
						onChange={(event) => onChange({ dScope: event.target.value })}
					>
						<Options
							values={[
								'Équipe directe',
								'Équipe et sous-équipes',
								"Toute l'organisation",
							]}
						/>
					</NativeSelect>
				</FieldRow>
				<FieldRow
					label="Profondeur maximale"
					hint="Un sous-agent peut-il déléguer à son tour"
				>
					<NativeSelect
						aria-label="Profondeur maximale"
						value={form.dDepth}
						onChange={(event) => onChange({ dDepth: event.target.value })}
					>
						<Options values={['1', '2', '3']} />
					</NativeSelect>
				</FieldRow>
				<FieldRow label="Sous-tâches en parallèle">
					<NativeSelect
						aria-label="Sous-tâches en parallèle"
						value={form.dPar}
						onChange={(event) => onChange({ dPar: event.target.value })}
					>
						<Options values={['1', '2', '3', '5', '10']} />
					</NativeSelect>
				</FieldRow>
				<FieldRow label="Budget par sous-tâche" hint="Plafond de dépense API">
					<NativeSelect
						aria-label="Budget par sous-tâche"
						value={form.dBudget}
						onChange={(event) => onChange({ dBudget: event.target.value })}
					>
						<Options values={['1 $', '5 $', '10 $', 'Illimité']} />
					</NativeSelect>
				</FieldRow>
				<FieldRow label="Résultats relus par">
					<NativeSelect
						aria-label="Résultats relus par"
						value={form.dReview}
						onChange={(event) => onChange({ dReview: event.target.value })}
					>
						<Options
							values={['Aucun (automatique)', "L'agent parent", 'Une personne']}
						/>
					</NativeSelect>
				</FieldRow>
				<FieldRow label="Si un sous-agent échoue">
					<NativeSelect
						aria-label="Si un sous-agent échoue"
						value={form.dFail}
						onChange={(event) => onChange({ dFail: event.target.value })}
					>
						<Options
							values={[
								'Réessayer 2 fois',
								"Remonter à l'agent parent",
								'Bloquer la tâche parente',
							]}
						/>
					</NativeSelect>
				</FieldRow>
				<p className="border-t px-3 py-2.5 text-muted-foreground">
					La tâche parente reste ouverte tant que ses sous-tâches le sont. Un
					sous-agent n'hérite d'aucun connecteur de son parent : chaque accès
					s'autorise séparément.
				</p>
			</SectionCard>

			<SectionCard title="Sous-tâches déléguées récemment" flush>
				{recent.length === 0 ? (
					<p className="px-3 py-4 text-muted-foreground">Aucune délégation.</p>
				) : (
					<ul className="divide-y">
						{recent.map(({ task, assignee }) => (
							<li key={task.id} className="flex items-center gap-2 px-3 py-2">
								<Link
									to="/tasks/$taskId"
									params={{ taskId: taskPathId(task.id) }}
								>
									<Badge variant="outline">{task.id}</Badge>
								</Link>
								<span className="min-w-0 flex-1">
									<b className="block truncate font-medium">{task.title}</b>
									<span className="text-xs text-muted-foreground">
										Parent {task.parentId}
									</span>
								</span>
								{assignee ? (
									<AgentAvatar
										name={assignee.name}
										hue={assignee.hue}
										size={20}
									/>
								) : null}
							</li>
						))}
					</ul>
				)}
			</SectionCard>
		</div>
	)
}
