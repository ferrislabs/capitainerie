import { useState } from 'react'
import { useNotice } from '#/components/layout/notice'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import {
	useApprovals,
	useDecideApproval,
} from '#/domain/approvals/hooks/use-approvals'
import {
	type ApprovalStatus,
	describeDecideError,
} from '#/domain/approvals/model'
import { ApprovalsView } from '#/domain/approvals/ui/approvals-view'
import { shortName } from '#/lib/agent-name'

export function ApprovalsPage() {
	const [status, setStatus] = useState<ApprovalStatus>('pending')
	const [decideError, setDecideError] = useState<string | null>(null)
	const [notice, notify] = useNotice()
	const query = useApprovals(status)
	const agents = useTeamAgents(false).data?.items
	const decide = useDecideApproval()

	return (
		<ApprovalsView
			status={status}
			approvals={query.data?.items ?? []}
			pendingCount={query.data?.pending ?? 0}
			loading={query.isPending}
			error={query.isError}
			notice={notice}
			decideError={decideError}
			busyId={
				decide.isPending ? (decide.variables?.path.approval_id ?? null) : null
			}
			requesterName={(id) => {
				const agent = id ? agents?.find((a) => a.id === id) : undefined
				return agent ? shortName(agent.name) : null
			}}
			onStatusChange={(next) => {
				setDecideError(null)
				setStatus(next)
			}}
			onRetry={() => void query.refetch()}
			onDecide={(id, decision, note) => {
				setDecideError(null)
				const trimmed = note.trim()
				decide.mutate(
					{
						path: { approval_id: id },
						body: { decision, ...(trimmed ? { note: trimmed } : {}) },
					},
					{
						onSuccess: () =>
							notify(
								decision === 'approve'
									? 'Approbation accordée'
									: 'Approbation refusée',
							),
						onError: (error) => setDecideError(describeDecideError(error)),
					},
				)
			}}
		/>
	)
}
