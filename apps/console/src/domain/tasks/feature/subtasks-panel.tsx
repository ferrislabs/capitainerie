import { Link } from '@tanstack/react-router'
import { useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
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
import { AgentAvatar } from '#/domain/tasks/ui/agent-chip'
import { StatusIcon } from '#/domain/tasks/ui/status-icon'
import { cn } from '#/lib/utils'

const CTL =
	'rounded-[6px] border bg-background px-[9px] py-[5px] text-foreground outline-none focus-visible:border-primary'

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
			className="my-2"
			actions={
				kids.length ? (
					<span className="rounded-lg bg-secondary px-1.5 text-[11px] font-normal text-muted-foreground">
						{done} / {kids.length} terminées
					</span>
				) : undefined
			}
		>
			{kids.length ? (
				<div>
					{kids.map((k) => {
						const agent = agentOf(k.agentId)
						return (
							<Link
								key={k.id}
								to="/tasks/$taskId"
								params={{ taskId: taskPathId(k.id) }}
								className="flex min-h-9 w-full min-w-0 items-center gap-2.5 border-t px-3 py-1.5 first:border-t-0 hover:bg-secondary"
							>
								<StatusIcon status={k.status} />
								<span className="font-mono text-[11px] font-medium whitespace-nowrap text-faint">
									{k.id}
								</span>
								<span className="min-w-0 flex-1">
									<b className="block truncate font-medium">{k.title}</b>
								</span>
								<AgentAvatar hue={agent.hue} size={20} label={agent.name} />
							</Link>
						)
					})}
				</div>
			) : (
				<p className="m-0 px-3 py-2.5 text-muted-foreground">
					Aucune sous-tâche pour l'instant.
				</p>
			)}
			{canDeleg ? (
				<form
					aria-label="Déléguer une sous-tâche"
					onSubmit={(event) => {
						event.preventDefault()
						delegate()
					}}
				>
					<div className="flex flex-wrap gap-1.5 px-3 pt-2.5 pb-3">
						<select
							aria-label="Sous-agent"
							value={subAgent || (subs[0] ?? '')}
							onChange={(event) => setSubAgent(event.target.value)}
							className={CTL}
						>
							{subs.map((id) => (
								<option key={id} value={id}>
									{agentOf(id).name}
								</option>
							))}
						</select>
						<input
							aria-label="Titre de la sous-tâche"
							placeholder="Sous-tâche à déléguer…"
							value={title}
							onChange={(event) => setTitle(event.target.value)}
							className={cn(CTL, 'min-w-0 flex-1')}
						/>
						<Button type="submit">Déléguer</Button>
					</div>
					<p className="m-0 px-3 pb-2.5 text-xs text-muted-foreground">
						Limites de {owner} : {fields.dPar} en parallèle, profondeur{' '}
						{fields.dDepth}, {fields.dBudget} par sous-tâche.
					</p>
				</form>
			) : null}
		</SectionCard>
	)
}
