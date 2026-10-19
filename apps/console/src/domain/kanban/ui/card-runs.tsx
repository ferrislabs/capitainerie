import { Link } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import type { Schemas } from '#/api/api.client'
import { StatusBadge } from '#/components/layout/status-badge'
import { isCardRun } from '#/domain/runs/run-title'
import { STATUS_LABELS, STATUS_TONES } from '#/domain/runs/status'

const dateFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'short',
	timeStyle: 'short',
})

interface Props {
	runs: ReadonlyArray<Schemas.RunResponse>
	runExtra: (run: Schemas.RunResponse) => ReactNode
}

export function CardRuns({ runs, runExtra }: Props) {
	return (
		<section aria-labelledby="card-runs-title" className="flex flex-col gap-3">
			<h3 id="card-runs-title" className="text-[15px] font-semibold">
				Runs liés
			</h3>
			{runs.length === 0 ? (
				<p className="text-[13px] text-muted-foreground">Aucun run lié.</p>
			) : (
				<ul className="flex flex-col divide-y rounded-md border bg-card">
					{runs.map((run) => (
						<li key={run.id} className="flex flex-col gap-1 px-3 py-2">
							<div className="flex items-center justify-between gap-2">
								<Link
									to="/runs/$runId"
									params={{ runId: run.id }}
									className="text-[13px] font-medium hover:text-link hover:underline"
								>
									{isCardRun(run)
										? `Exécution sur ${run.repository ?? 'un dépôt'}`
										: 'Run d’origine'}
								</Link>
								<StatusBadge
									tone={STATUS_TONES[run.status]}
									label={STATUS_LABELS[run.status]}
								/>
							</div>
							<time
								dateTime={run.created_at}
								className="text-xs text-muted-foreground figures"
							>
								{dateFormat.format(new Date(run.created_at))}
							</time>
							{runExtra(run)}
						</li>
					))}
				</ul>
			)}
		</section>
	)
}
