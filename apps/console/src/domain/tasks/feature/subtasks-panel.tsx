import { Link } from '@tanstack/react-router'
import { useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import { TextInput } from '#/components/ui/text-input'
import { getAgentConfig } from '#/domain/agent-team/agent-config-store'
import type { DemoTask } from '#/domain/demo'
import {
	addTask,
	childTasks,
	demoStore,
	nextTaskId,
	subAgentIds,
	taskDepth,
	useDemoState,
} from '#/domain/demo'
import { canDelegate, openChildCount } from '#/domain/tasks/delegation'
import { useAgentOf } from '#/domain/tasks/feature/use-agent-of'
import { taskPathId } from '#/domain/tasks/task-ids'
import { AgentChip } from '#/domain/tasks/ui/agent-chip'
import { StatusIcon } from '#/domain/tasks/ui/status-icon'

interface Props {
	task: DemoTask
	onNotify: (message: string) => void
}

export function SubtasksPanel({ task, onNotify }: Props) {
	const state = useDemoState((s) => s)
	const agentOf = useAgentOf()
	const [subAgent, setSubAgent] = useState('')
	const [title, setTitle] = useState('')
	const kids = childTasks(state, task.id)
	const subs = subAgentIds(state, task.agentId)
	const fields = getAgentConfig(task.agentId).fields
	const canDeleg = fields.canDeleg && subs.length > 0
	if (kids.length === 0 && !canDeleg) return null
	const done = kids.filter((k) => k.status === 'done').length
	const owner = agentOf(task.agentId).name

	const delegate = () => {
		const result = canDelegate({
			title,
			depth: taskDepth(demoStore.state, task.id),
			openChildren: openChildCount(childTasks(demoStore.state, task.id)),
			maxDepth: Number(fields.dDepth),
			maxParallel: Number(fields.dPar),
		})
		if (!result.ok) {
			onNotify(result.reason)
			return
		}
		const target = subAgent || (subs[0] ?? '')
		const id = nextTaskId(demoStore.state)
		addTask({
			id,
			title: title.trim(),
			status: 'todo',
			agentId: target,
			age: "à l'instant",
			project: task.project,
			parentId: task.id,
		})
		setTitle('')
		onNotify(`${id} déléguée à ${agentOf(target).name}`)
	}

	return (
		<SectionCard
			title="Sous-tâches"
			flush
			actions={
				kids.length ? (
					<span className="text-xs text-muted-foreground">
						{done} / {kids.length} terminées
					</span>
				) : undefined
			}
		>
			{kids.length ? (
				<ul className="divide-y">
					{kids.map((k) => {
						const agent = agentOf(k.agentId)
						return (
							<li
								key={k.id}
								className="flex items-center gap-3 px-3 py-2 text-[13px]"
							>
								<StatusIcon status={k.status} />
								<Link
									to="/tasks/$taskId"
									params={{ taskId: taskPathId(k.id) }}
									className="font-mono text-xs text-signal-running hover:underline"
								>
									{k.id}
								</Link>
								<span className="min-w-0 flex-1 truncate">{k.title}</span>
								<AgentChip
									name={agent.name}
									hue={agent.hue}
									showName={false}
									size={20}
								/>
							</li>
						)
					})}
				</ul>
			) : (
				<p className="px-3 py-2.5 text-[13px] text-muted-foreground">
					Aucune sous-tâche pour l'instant.
				</p>
			)}
			{canDeleg ? (
				<form
					className="flex flex-col gap-2 border-t p-3"
					aria-label="Déléguer une sous-tâche"
					onSubmit={(event) => {
						event.preventDefault()
						delegate()
					}}
				>
					<div className="flex flex-wrap items-center gap-2">
						<NativeSelect
							aria-label="Sous-agent"
							value={subAgent || (subs[0] ?? '')}
							onChange={(event) => setSubAgent(event.target.value)}
						>
							{subs.map((id) => (
								<option key={id} value={id}>
									{agentOf(id).name}
								</option>
							))}
						</NativeSelect>
						<TextInput
							aria-label="Titre de la sous-tâche"
							placeholder="Sous-tâche à déléguer…"
							value={title}
							onChange={(event) => setTitle(event.target.value)}
							wrapperClassName="min-w-48 flex-1"
						/>
						<Button type="submit">Déléguer</Button>
					</div>
					<p className="text-xs text-muted-foreground">
						Limites de {owner} : {fields.dPar} en parallèle, profondeur{' '}
						{fields.dDepth}, {fields.dBudget} par sous-tâche.
					</p>
				</form>
			) : null}
		</SectionCard>
	)
}
