import { useState } from 'react'
import {
	useActiveRuns,
	useAgentAnalytics,
	useDailyCosts,
	useDashboardCards,
	useRecentRuns,
} from '#/domain/dashboard/hooks/use-dashboard-data'
import { DashboardView } from '#/domain/dashboard/ui/dashboard-view'
import { NewTaskDialog } from '#/domain/tasks/feature/new-task-dialog'

export function DashboardPage() {
	const [newTaskOpen, setNewTaskOpen] = useState(false)
	const active = useActiveRuns()
	const runs = useRecentRuns()
	const { query: costs, days } = useDailyCosts()
	const cards = useDashboardCards()
	const agents = useAgentAnalytics()
	return (
		<>
			<DashboardView
				active={active}
				runs={runs}
				costs={costs}
				days={days}
				cards={cards}
				agents={agents}
				onNewTask={() => setNewTaskOpen(true)}
			/>
			<NewTaskDialog open={newTaskOpen} onOpenChange={setNewTaskOpen} />
		</>
	)
}
