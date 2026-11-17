import {
	ACTIVITY,
	COST_TILES,
	DAILY_RUNS,
} from '#/domain/audit/mock/audit-data'
import { currentTask, dashboardStats } from '#/domain/dashboard/stats'
import { DashboardView } from '#/domain/dashboard/ui/dashboard-view'
import { shortName, useDemoState } from '#/domain/demo'

export function DashboardPage() {
	const agents = useDemoState((s) => s.agents)
	const tasks = useDemoState((s) => s.tasks)
	const approvals = useDemoState((s) => s.approvals)
	const liveAgents = agents
		.filter((a) => a.status === 'running')
		.map((a) => {
			const task = currentTask(tasks, a.id)
			return {
				id: a.id,
				name: shortName(a.name),
				task: task?.title ?? null,
				taskId: task?.id ?? null,
			}
		})
	const spend = COST_TILES.find((t) => t.label === 'Dépense API')
	const budget = COST_TILES.find((t) => t.label === 'Budget utilisé')
	return (
		<DashboardView
			stats={dashboardStats(agents, tasks, approvals)}
			liveAgents={liveAgents}
			days={DAILY_RUNS}
			events={ACTIVITY.slice(0, 6)}
			spend={spend?.value ?? '0,00 $'}
			budget={budget?.hint ?? ''}
		/>
	)
}
