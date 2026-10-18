import { ExternalLink } from 'lucide-react'
import type { ExecuteOutput } from '#/domain/runs/execute-output'

interface Props {
	output: ExecuteOutput
	failed: boolean
	stepError: string | null | undefined
}

export function droppedToolCallsText(count: number): string {
	return count === 1
		? "1 appel d'outil non affiché"
		: `${count} appels d'outils non affichés`
}

export function ExecuteOutcome({ output, failed, stepError }: Props) {
	const reason = failed && output.error !== stepError ? output.error : undefined

	return (
		<div className="flex flex-col gap-1 text-[13px]">
			{output.pullRequest ? (
				<a
					href={output.pullRequest}
					target="_blank"
					rel="noreferrer"
					className="inline-flex w-fit items-center gap-1 font-medium text-link hover:underline"
				>
					Pull request en brouillon
					<ExternalLink aria-hidden className="size-3.5" />
				</a>
			) : null}
			{output.branch ? (
				<p className="text-muted-foreground">
					Branche{' '}
					<span className="font-mono text-xs text-foreground">
						{output.branch}
					</span>
				</p>
			) : null}
			{output.noChanges ? (
				<p className="text-muted-foreground">Aucun changement</p>
			) : null}
			{reason ? (
				<p role="alert" className="font-medium text-signal-danger">
					{reason}
				</p>
			) : null}
			{output.droppedToolCalls > 0 ? (
				<p className="text-xs text-muted-foreground figures">
					{droppedToolCallsText(output.droppedToolCalls)}
				</p>
			) : null}
		</div>
	)
}
