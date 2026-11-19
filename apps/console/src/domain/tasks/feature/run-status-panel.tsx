import { Loader2 } from 'lucide-react'
import { useEffect } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import {
	useRefreshCard,
	useRunPolling,
	useRunTimeline,
} from '#/domain/tasks/hooks/use-agent-run'
import { isActiveRun, RUN_STATUS_LABEL } from '#/domain/tasks/run-agent'
import { AgentAvatar } from '#/domain/tasks/ui/agent-chip'

interface Props {
	runId: string
	agentName: string
	agentHue: number
	onActiveChange: (active: boolean) => void
}

/**
 * L'exécution en cours, comme une ligne du fil : l'agent travaille, avec le
 * dernier outil appelé ; puis, une fois fini, les appels faits.
 */
export function RunStatusPanel({
	runId,
	agentName,
	agentHue,
	onActiveChange,
}: Props) {
	const run = useRunPolling(runId)
	const refresh = useRefreshCard()
	const status = run.data?.status
	const active = status === undefined || isActiveRun(status)
	const finished = !active
	const timeline = useRunTimeline(runId, true, active)
	const calls = (timeline.data?.steps ?? []).flatMap((step) =>
		step.tool_calls.map((call, index) => ({
			...call,
			key: `${step.seq}-${index}`,
		})),
	)
	const last = calls.at(-1)

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
	return (
		<section
			aria-label="Exécution de l'agent"
			className="grid grid-cols-[28px_minmax(0,1fr)] gap-x-3"
		>
			<AgentAvatar hue={agentHue} size={28} />
			<div className="min-w-0 pt-1">
				<p className="m-0 flex flex-wrap items-center gap-x-2 text-[14px]">
					{active ? (
						<Loader2
							aria-hidden="true"
							className="size-3.5 animate-spin text-muted-foreground"
						/>
					) : null}
					<b className="font-semibold">{agentName}</b>
					<span className="text-muted-foreground">
						{status ? RUN_STATUS_LABEL[status] : 'Chargement de l’exécution…'}
					</span>
					{active && last ? (
						<span className="min-w-0 truncate font-mono text-[12.5px] text-muted-foreground">
							· {last.tool}
						</span>
					) : null}
				</p>
				{status === 'failed' ? (
					<InlineAlert tone="danger" className="mt-2">
						{run.data?.failure ?? "L'exécution a échoué."}
					</InlineAlert>
				) : null}
				{finished && calls.length ? (
					<details className="mt-1 text-[13px] text-muted-foreground">
						<summary className="cursor-pointer select-none">
							{calls.length} appel{calls.length > 1 ? 's' : ''} d'outils
						</summary>
						<ul aria-label="Appels d'outils" className="m-0 mt-1 list-none p-0">
							{calls.map((call) => (
								<li key={call.key} className="text-xs">
									{call.succeeded ? '✓' : '✗'} {call.tool} : {call.summary}
								</li>
							))}
						</ul>
					</details>
				) : null}
			</div>
		</section>
	)
}
