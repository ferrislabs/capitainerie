import { useState } from 'react'
import { useApprovals } from '#/domain/approvals/hooks/use-approvals'
import {
	useActiveRuns,
	useAgentAnalytics,
	useDailyCosts,
	useDashboardCards,
	useRecentRuns,
} from '#/domain/dashboard/hooks/use-dashboard-data'
import { DashboardView } from '#/domain/dashboard/ui/dashboard-view'
import { useInboxUnread } from '#/domain/inbox/hooks/use-inbox'
import { NewTaskDialog } from '#/domain/tasks/feature/new-task-dialog'

export function DashboardPage() {
	const [newTaskOpen, setNewTaskOpen] = useState(false)
	const active = useActiveRuns()
	const runs = useRecentRuns()
	const { query: costs, days } = useDailyCosts()
	const cards = useDashboardCards()
	const agents = useAgentAnalytics()
	const approvals = useApprovals('pending')
	const inbox = useInboxUnread()
	return (
		<>
			<DashboardView
				active={active}
				runs={runs}
				costs={costs}
				days={days}
				cards={cards}
				agents={agents}
				approvals={approvals}
				inbox={inbox}
				onNewTask={() => setNewTaskOpen(true)}
			/>
			<NewTaskDialog open={newTaskOpen} onOpenChange={setNewTaskOpen} />
		</>
	)
}
