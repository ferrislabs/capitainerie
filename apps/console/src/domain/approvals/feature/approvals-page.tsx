import { useNotice } from '#/components/layout/notice'
import { ApprovalsView } from '#/domain/approvals/ui/approvals-view'
import { decideApproval, shortName, useDemoState } from '#/domain/demo'

export function ApprovalsPage() {
	const approvals = useDemoState((s) => s.approvals)
	const agents = useDemoState((s) => s.agents)
	const [notice, notify] = useNotice()
	return (
		<ApprovalsView
			pending={approvals.filter((a) => a.status === 'pending')}
			decided={approvals.filter((a) => a.status !== 'pending')}
			requesterName={(id) => {
				const agent = agents.find((a) => a.id === id)
				return agent ? shortName(agent.name) : null
			}}
			notice={notice}
			onDecide={(id, status) => {
				decideApproval(id, status, 'Board')
				notify(
					status === 'approved'
						? 'Approbation accordée'
						: 'Approbation refusée',
				)
			}}
		/>
	)
}
