import { formatRun } from '#/domain/routines/schedule'

interface Props {
	runs: ReadonlyArray<Date>
	empty: string
}

export function NextRuns({ runs, empty }: Props) {
	if (runs.length === 0)
		return <p className="text-[13px] text-muted-foreground">{empty}</p>
	return (
		<ul className="flex flex-col gap-1 text-[13px]">
			{runs.map((run) => (
				<li key={run.getTime()} className="font-medium">
					{formatRun(run)}
				</li>
			))}
		</ul>
	)
}

export const CRON_NOTE =
	"Les prochains passages d'une expression cron sont calculés par le serveur."
