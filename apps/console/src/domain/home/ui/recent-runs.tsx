import { Link } from '@tanstack/react-router'
import { Play } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { EmptyState } from '#/components/layout/empty-state'
import { StatusBadge } from '#/components/layout/status-badge'
import { Button } from '#/components/ui/button'
import { runTitle } from '#/domain/runs/run-title'
import { STATUS_LABELS, STATUS_TONES } from '#/domain/runs/status'

export const RECENT_RUNS_COUNT = 5

const dateFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'short',
	timeStyle: 'short',
})

interface Props {
	runs: ReadonlyArray<Schemas.RunResponse>
}

export function RecentRuns({ runs }: Props) {
	if (runs.length === 0) {
		return (
			<EmptyState
				icon={Play}
				title="Aucune activité"
				description="Les runs apparaîtront ici dès le premier dépôt."
				action={
					<Button asChild variant="outline">
						<Link to="/runs">Déposer une réunion</Link>
					</Button>
				}
			/>
		)
	}

	return (
		<ul className="divide-y">
			{runs.slice(0, RECENT_RUNS_COUNT).map((run) => (
				<li
					key={run.id}
					className="flex min-h-10 items-center gap-3 px-4 py-2 text-[13px]"
				>
					<Link
						to="/runs/$runId"
						params={{ runId: run.id }}
						className="min-w-0 flex-1 truncate font-medium hover:text-link hover:underline"
					>
						{runTitle(run)}
					</Link>
					<StatusBadge
						tone={STATUS_TONES[run.status]}
						label={STATUS_LABELS[run.status]}
					/>
					<time
						dateTime={run.created_at}
						className="figures hidden shrink-0 text-muted-foreground sm:block"
					>
						{dateFormat.format(new Date(run.created_at))}
					</time>
				</li>
			))}
		</ul>
	)
}
