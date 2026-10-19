import { ExternalLink } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { parseExecuteOutput } from '#/domain/runs/execute-output'
import { useRunTimeline } from '#/domain/runs/hooks/use-run-timeline'

interface Props {
	runId: string
}

function ExecutionPullRequestLink({ runId }: Props) {
	const timeline = useRunTimeline(runId)
	const step = timeline.data?.steps.find(
		(candidate) => candidate.kind === 'execute',
	)
	const pullRequest = step
		? parseExecuteOutput(step.execution)?.pullRequest
		: undefined
	if (!pullRequest) return null

	return (
		<a
			href={pullRequest}
			target="_blank"
			rel="noreferrer"
			className="inline-flex w-fit items-center gap-1 text-xs font-medium text-link hover:underline"
		>
			Pull request en brouillon
			<ExternalLink aria-hidden className="size-3.5" />
		</a>
	)
}

export function executionPullRequest(run: Schemas.RunResponse) {
	if (run.card_id == null || run.status === 'queued') return null
	return <ExecutionPullRequestLink runId={run.id} />
}
