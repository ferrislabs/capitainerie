import { useEffect } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import {
	useRefreshCard,
	useRunPolling,
	useRunTimeline,
} from '#/domain/tasks/hooks/use-agent-run'
import { isActiveRun, RUN_STATUS_LABEL } from '#/domain/tasks/run-agent'

interface Props {
	runId: string
	onActiveChange: (active: boolean) => void
}

export function RunStatusPanel({ runId, onActiveChange }: Props) {
	const run = useRunPolling(runId)
	const refresh = useRefreshCard()
	const status = run.data?.status
	const finished = status !== undefined && !isActiveRun(status)
	const timeline = useRunTimeline(runId, finished)
	const calls = (timeline.data?.steps ?? []).flatMap((step) =>
		step.tool_calls.map((call, index) => ({
			...call,
			key: `${step.seq}-${index}`,
		})),
	)

	useEffect(() => {
		if (finished) {
			refresh()
			onActiveChange(false)
		}
	}, [finished, refresh, onActiveChange])

	if (run.isError) {
		return (
			<InlineAlert tone="danger">
				Impossible de suivre l'exécution de l'agent.
			</InlineAlert>
		)
	}
	if (!status) {
		return (
			<output className="block rounded-lg border bg-card px-3 py-2">
				Chargement de l'exécution…
			</output>
		)
	}
	return (
		<section
			aria-label="Exécution de l'agent"
			className="rounded-lg border bg-card px-3 py-2"
		>
			<p className="m-0 font-medium">
				Exécution de l'agent : <span>{RUN_STATUS_LABEL[status]}</span>
			</p>
			{status === 'failed' ? (
				<InlineAlert tone="danger" className="mt-2">
					{run.data?.failure ?? "L'exécution a échoué."}
				</InlineAlert>
			) : null}
			{finished && calls.length ? (
				<ul aria-label="Appels d'outils" className="m-0 mt-2 list-none p-0">
					{calls.map((call) => (
						<li key={call.key} className="text-xs text-muted-foreground">
							{call.succeeded ? '✓' : '✗'} {call.tool} : {call.summary}
						</li>
					))}
				</ul>
			) : null}
		</section>
	)
}
