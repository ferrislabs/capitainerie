import { Link } from '@tanstack/react-router'
import type { TeamAgent } from '#/domain/agent-team/team-agent'
import { AgentAvatar } from '#/domain/agent-team/ui/agent-avatar'
import { DETAIL_ROW, Tag, Ttl } from '#/domain/agent-team/ui/rows'
import type { Task } from '#/domain/tasks/model'
import { taskPathId } from '#/domain/tasks/task-ids'
import { StatusIcon } from '#/domain/tasks/ui/status-icon'
import { shortName } from '#/lib/agent-name'

interface Props {
	task: Task
	assignee: Pick<TeamAgent, 'hue' | 'name'> | undefined
	sub?: string
	showName?: boolean
}

export function TaskLine({ task, assignee, sub, showName = true }: Props) {
	return (
		<Link
			to="/tasks/$taskId"
			params={{ taskId: taskPathId(task.id) }}
			className={DETAIL_ROW}
		>
			<StatusIcon status={task.status} />
			<Tag>{task.shortId}</Tag>
			<Ttl title={task.title} sub={sub ?? task.note ?? undefined} />
			{assignee ? (
				<span className="flex min-w-0 items-center gap-1.5 whitespace-nowrap text-xs text-muted-foreground">
					<AgentAvatar hue={assignee.hue} size={showName ? 18 : 20} />
					{showName ? (
						<span className="hidden md:inline">{shortName(assignee.name)}</span>
					) : null}
				</span>
			) : null}
			{showName ? (
				<span className="whitespace-nowrap font-mono text-[11.5px] text-faint">
					{task.age}
				</span>
			) : null}
		</Link>
	)
}
