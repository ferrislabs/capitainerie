import { formatRun } from '#/domain/routines/schedule'
import { cn } from '#/lib/utils'

interface Props {
	runs: ReadonlyArray<Date>
	empty: string
	emptyClassName?: string
}

export function NextRuns({ runs, empty, emptyClassName }: Props) {
	if (runs.length === 0)
		return (
			<p
				className={cn('m-0 px-3 py-2.5 text-muted-foreground', emptyClassName)}
			>
				{empty}
			</p>
		)
	return (
		<div>
			{runs.map((run) => (
				<div key={run.getTime()} className="px-3 py-1.5">
					<b>{formatRun(run)}</b>
				</div>
			))}
		</div>
	)
}

export const CRON_NOTE =
	"Les prochains passages d'une expression cron sont calculés par le serveur."
