import { useState } from 'react'
import { dashboardStats } from '#/domain/dashboard/stats'
import { DashboardView } from '#/domain/dashboard/ui/dashboard-view'
import { useDemoState } from '#/domain/demo'
import { NewTaskDialog } from '#/domain/tasks/feature/new-task-dialog'

const RECENT_TASKS = 7

export function DashboardPage() {
	const agents = useDemoState((s) => s.agents)
	const tasks = useDemoState((s) => s.tasks)
	const approvals = useDemoState((s) => s.approvals)
	const [newTaskOpen, setNewTaskOpen] = useState(false)
	const { pending } = dashboardStats(agents, tasks, approvals)
	return (
		<>
			<DashboardView
				agents={agents}
				recentTasks={tasks.slice(0, RECENT_TASKS)}
				pendingApprovals={pending}
				onNewTask={() => setNewTaskOpen(true)}
			/>
			<NewTaskDialog open={newTaskOpen} onOpenChange={setNewTaskOpen} />
		</>
	)
}
