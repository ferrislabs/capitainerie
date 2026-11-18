import { Link } from '@tanstack/react-router'
import { useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import {
	canDelegate,
	MAX_PARALLEL_SUBTASKS,
	MAX_SUBTASK_DEPTH,
	openChildCount,
} from '#/domain/tasks/delegation'
import { useAgentOf } from '#/domain/tasks/feature/use-agent-of'
import { useCreateCard } from '#/domain/tasks/hooks/use-tasks'
import { ancestorCount, childrenOf, type Task } from '#/domain/tasks/model'
import { taskPathId } from '#/domain/tasks/task-ids'
import { AgentAvatar } from '#/domain/tasks/ui/agent-chip'
import { StatusIcon } from '#/domain/tasks/ui/status-icon'
import { cn } from '#/lib/utils'

const CTL =
	'rounded-[6px] border bg-background px-[9px] py-[5px] text-foreground outline-none focus-visible:border-primary'

interface Props {
	task: Task
	tasks: ReadonlyArray<Task>
	onNotify: (message: string) => void
}

export function SubtasksPanel({ task, tasks, onNotify }: Props) {
	const agents = useTeamAgents().data?.items ?? []
	const agentOf = useAgentOf()
	const create = useCreateCard()
	const [subAgent, setSubAgent] = useState('')
	const [title, setTitle] = useState('')
	const kids = childrenOf(tasks, task.id)
	const subs = task.agentId
		? agents.filter((a) => a.reports_to === task.agentId)
		: []
	const canDeleg = subs.length > 0
	if (kids.length === 0 && !canDeleg) return null
	const done = kids.filter((k) => k.status === 'done').length
	const owner = agentOf(task.agentId).name

	const delegate = () => {
		const result = canDelegate({
			title,
			depth: ancestorCount(tasks, task.id),
			openChildren: openChildCount(kids),
			maxDepth: MAX_SUBTASK_DEPTH,
			maxParallel: MAX_PARALLEL_SUBTASKS,
		})
		if (!result.ok) {
			onNotify(result.reason)
			return
		}
		const target = subAgent || (subs[0]?.id ?? '')
		create.mutate(
			{
				body: {
					title: title.trim(),
					assignee_agent_id: target,
					project_id: task.projectId,
					parent_id: task.id,
				},
			},
			{
				onSuccess: (card) => {
					setTitle('')
					onNotify(`${card.id.slice(0, 8)} déléguée à ${agentOf(target).name}`)
				},
				onError: () => onNotify('La délégation a échoué'),
			},
		)
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
									{k.shortId}
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
							value={subAgent || (subs[0]?.id ?? '')}
							onChange={(event) => setSubAgent(event.target.value)}
							className={CTL}
						>
							{subs.map((a) => (
								<option key={a.id} value={a.id}>
									{' '}
									{agentOf(a.id).name}{' '}
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
						<Button type="submit" disabled={create.isPending}>
							Déléguer
						</Button>
					</div>
					<p className="m-0 px-3 pb-2.5 text-xs text-muted-foreground">
						Limites de {owner} : {MAX_PARALLEL_SUBTASKS} en parallèle,
						profondeur {MAX_SUBTASK_DEPTH}.
					</p>
				</form>
			) : null}
		</SectionCard>
	)
}
