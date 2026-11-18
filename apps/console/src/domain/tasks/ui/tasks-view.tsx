import { Link } from '@tanstack/react-router'
import { Plus } from 'lucide-react'
import type { Task } from '#/domain/tasks/model'
import { taskPathId } from '#/domain/tasks/task-ids'
import type { TaskGroup } from '#/domain/tasks/task-list'
import { AgentChip } from '#/domain/tasks/ui/agent-chip'
import { StatusIcon } from '#/domain/tasks/ui/status-icon'

export interface AgentView {
	name: string
	fullName: string
	hue: number
	model: string
}

interface Props {
	groups: TaskGroup[]
	agentOf: (agentId: string | null) => AgentView
	onAdd?: () => void
}

const TAG = 'font-mono text-[11px] font-medium whitespace-nowrap text-faint'

function TaskRow({ task, agent }: { task: Task; agent: AgentView }) {
	return (
		<Link
			to="/tasks/$taskId"
			params={{ taskId: taskPathId(task.id) }}
			className="flex min-h-9 w-full min-w-0 items-center gap-2.5 border-t px-3 py-1.5 first:border-t-0 hover:bg-secondary"
		>
			<StatusIcon status={task.status} />
			<span className={TAG}>{task.shortId}</span>
			<span className="min-w-0 flex-1">
				<b className="block truncate font-medium leading-[19px]">
					{task.title}
				</b>
				{task.note ? (
					<span className="block truncate text-xs leading-[1.4] text-muted-foreground">
						{task.note}
					</span>
				) : null}
			</span>
			<AgentChip name={agent.name} hue={agent.hue} />
			<span className="font-mono text-[11.5px] whitespace-nowrap text-faint">
				{task.age}
			</span>
		</Link>
	)
}

export function TasksList({ groups, agentOf }: Props) {
	return (
		<>
			{groups.map((group) => (
				<section key={group.key} aria-label={group.label}>
					<div className="flex items-center gap-2 rounded-t-[7px] border bg-secondary px-3 py-1.5 text-xs font-semibold">
						{group.status ? <StatusIcon status={group.status} /> : null}
						{group.label}
						<span className="text-muted-foreground tabular-nums">
							{group.tasks.length}
						</span>
					</div>
					<div className="mb-3 overflow-hidden rounded-b-[7px] border border-t-0 bg-card">
						{group.tasks.map((task) => (
							<TaskRow
								key={task.id}
								task={task}
								agent={agentOf(task.agentId)}
							/>
						))}
					</div>
				</section>
			))}
		</>
	)
}

export function TasksBoard({ groups, agentOf, onAdd }: Props) {
	return (
		<div className="grid auto-cols-[minmax(240px,1fr)] grid-flow-col gap-2.5 overflow-x-auto pb-2">
			{groups.map((group) => (
				<section
					key={group.key}
					aria-label={group.label}
					className="flex min-w-0 flex-col rounded-lg border bg-card"
				>
					<h3 className="m-0 flex items-center gap-[7px] px-2.5 py-2 text-xs font-semibold">
						{group.status ? <StatusIcon status={group.status} /> : null}
						{group.label}
						<span className="text-muted-foreground tabular-nums">
							{group.tasks.length}
						</span>
						<span className="flex-1" />
						<button
							type="button"
							aria-label="Ajouter"
							onClick={onAdd}
							className="grid size-[22px] place-items-center rounded-md text-muted-foreground hover:bg-secondary hover:text-foreground"
						>
							<Plus aria-hidden="true" className="size-4" />
						</button>
					</h3>
					{group.tasks.map((task) => {
						const agent = agentOf(task.agentId)
						return (
							<Link
								key={task.id}
								to="/tasks/$taskId"
								params={{ taskId: taskPathId(task.id) }}
								className="mx-2 mb-2 block rounded-[7px] border bg-background px-[9px] py-2 hover:border-faint"
							>
								<span className={TAG}>{task.shortId}</span>
								<div className="mt-[3px] mb-[7px] line-clamp-2 font-medium">
									{task.title}
								</div>
								<div className="flex items-center justify-between gap-2">
									<AgentChip name={agent.name} hue={agent.hue} />
									<span className="font-mono text-[11.5px] text-faint">
										{task.age.replace('il y a ', '')}
									</span>
								</div>
							</Link>
						)
					})}
				</section>
			))}
		</div>
	)
}
