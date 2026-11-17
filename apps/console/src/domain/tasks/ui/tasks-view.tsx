import { Link } from '@tanstack/react-router'
import { taskPathId } from '#/domain/tasks/task-ids'
import { STATUS_LABEL, type TaskGroup } from '#/domain/tasks/task-list'
import { AgentChip } from '#/domain/tasks/ui/agent-chip'
import { StatusIcon } from '#/domain/tasks/ui/status-icon'

export interface AgentView {
	name: string
	hue: number
}

interface Props {
	groups: TaskGroup[]
	agentOf: (agentId: string) => AgentView
}

export function TasksList({ groups, agentOf }: Props) {
	return (
		<div className="flex flex-col gap-4">
			{groups.map((group) => (
				<section key={group.status} aria-label={STATUS_LABEL[group.status]}>
					<h2 className="mb-1.5 flex items-center gap-2 text-[13px] font-medium">
						<StatusIcon status={group.status} />
						{STATUS_LABEL[group.status]}
						<span className="text-muted-foreground tabular-nums">
							{group.tasks.length}
						</span>
					</h2>
					<ul className="divide-y overflow-hidden rounded-lg border bg-card">
						{group.tasks.map((task) => {
							const agent = agentOf(task.agentId)
							return (
								<li key={task.id}>
									<Link
										to="/tasks/$taskId"
										params={{ taskId: taskPathId(task.id) }}
										className="flex items-center gap-3 px-3 py-2 text-[13px] hover:bg-muted/50"
									>
										<StatusIcon status={task.status} />
										<span className="font-mono text-xs text-muted-foreground">
											{task.id}
										</span>
										<span className="min-w-0 flex-1">
											<span className="block truncate font-medium">
												{task.title}
											</span>
											<span className="block truncate text-xs text-muted-foreground">
												{task.project || 'Aucun projet'}
											</span>
										</span>
										<AgentChip name={agent.name} hue={agent.hue} />
										<span className="text-xs text-muted-foreground">
											{task.age}
										</span>
									</Link>
								</li>
							)
						})}
					</ul>
				</section>
			))}
		</div>
	)
}

export function TasksBoard({ groups, agentOf }: Props) {
	return (
		<div className="grid auto-cols-[16rem] grid-flow-col gap-3 overflow-x-auto pb-2">
			{groups.map((group) => (
				<section
					key={group.status}
					aria-label={STATUS_LABEL[group.status]}
					className="flex flex-col gap-2 rounded-lg bg-muted/40 p-2"
				>
					<h2 className="flex items-center gap-2 text-[13px] font-medium">
						<StatusIcon status={group.status} />
						{STATUS_LABEL[group.status]}
						<span className="text-muted-foreground tabular-nums">
							{group.tasks.length}
						</span>
					</h2>
					{group.tasks.map((task) => {
						const agent = agentOf(task.agentId)
						return (
							<Link
								key={task.id}
								to="/tasks/$taskId"
								params={{ taskId: taskPathId(task.id) }}
								className="flex flex-col gap-1.5 rounded-md border bg-card p-2.5 text-[13px] hover:bg-muted/50"
							>
								<span className="font-mono text-xs text-muted-foreground">
									{task.id}
								</span>
								<span className="line-clamp-3 font-medium">{task.title}</span>
								<span className="flex items-center justify-between gap-2">
									<AgentChip name={agent.name} hue={agent.hue} />
									<span className="text-xs text-muted-foreground">
										{task.age}
									</span>
								</span>
							</Link>
						)
					})}
				</section>
			))}
		</div>
	)
}
