import { useState } from 'react'
import {
	useApprovals,
	useDecideApproval,
} from '#/domain/approvals/hooks/use-approvals'
import {
	type DecisionOutcome,
	decisionOutcome,
	describeDecideError,
} from '#/domain/approvals/model'
import { ApprovalCard } from '#/domain/approvals/ui/approval-card'
import { useRefreshCard } from '#/domain/tasks/hooks/use-agent-run'

interface Props {
	cardId: string
	agentName: string
}

/** Le plan qu'un agent attend de voir appliqué sur cette tâche, avec la décision. */
export function TaskPlanPanel({ cardId, agentName }: Props) {
	const query = useApprovals('pending')
	const decide = useDecideApproval()
	const refreshCard = useRefreshCard()
	const [outcome, setOutcome] = useState<DecisionOutcome | null>(null)
	const [error, setError] = useState<string | null>(null)
	const plans = (query.data?.items ?? []).filter(
		(approval) =>
			approval.card_id === cardId && approval.kind === 'repository_change',
	)
	if (plans.length === 0 && !outcome && !error) return null

	return (
		<section aria-label="Plan proposé" className="grid gap-2">
			{outcome ? (
				<output className="block rounded-md border bg-card px-3 py-2 text-[13px]">
					{outcome.message}
				</output>
			) : null}
			{error ? (
				<p
					role="alert"
					className="m-0 rounded-md border border-destructive/50 px-3 py-2 text-[13px] text-destructive"
				>
					{error}
				</p>
			) : null}
			{plans.map((approval) => (
				<ApprovalCard
					key={approval.id}
					approval={approval}
					requester={agentName}
					busy={decide.isPending}
					onDecide={(id, decision, note) => {
						setOutcome(null)
						setError(null)
						const trimmed = note.trim()
						decide.mutate(
							{
								path: { approval_id: id },
								body: { decision, ...(trimmed ? { note: trimmed } : {}) },
							},
							{
								onSuccess: (decided) => {
									setOutcome(decisionOutcome(decision, decided))
									void refreshCard()
								},
								onError: (failure) => setError(describeDecideError(failure)),
							},
						)
					}}
				/>
			))}
		</section>
	)
}
