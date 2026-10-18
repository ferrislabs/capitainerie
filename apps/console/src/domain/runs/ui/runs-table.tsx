import { Link } from '@tanstack/react-router'
import { Play } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { EmptyState } from '#/components/layout/empty-state'
import { StatusBadge } from '#/components/layout/status-badge'
import { Badge } from '#/components/ui/badge'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import { isCardRun, runTitle } from '#/domain/runs/run-title'
import { STATUS_LABELS, STATUS_TONES } from '#/domain/runs/status'

const dateFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'short',
	timeStyle: 'short',
})

interface Props {
	runs: ReadonlyArray<Schemas.RunResponse>
}

export function RunsTable({ runs }: Props) {
	if (runs.length === 0) {
		return (
			<EmptyState
				icon={Play}
				title="Aucun run"
				description="Déposez un transcript pour lancer un premier run."
			/>
		)
	}

	return (
		<Table containerClassName="max-h-[calc(100dvh-14rem)] min-h-48">
			<TableHeader>
				<TableRow>
					<TableHead>Titre</TableHead>
					<TableHead>Statut</TableHead>
					<TableHead>Lancé le</TableHead>
					<TableHead className="hidden 2xl:table-cell">Étapes</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{runs.map((run) => (
					<TableRow key={run.id}>
						<TableCell className="max-w-[24rem]">
							<div className="flex min-w-0 items-baseline gap-2">
								<Link
									to="/runs/$runId"
									params={{ runId: run.id }}
									className="truncate font-medium text-foreground hover:text-link hover:underline"
								>
									{runTitle(run)}
								</Link>
								<span
									className="shrink-0 font-mono text-[11px] text-muted-foreground"
									title={run.id}
								>
									{run.id.slice(0, 8)}
								</span>
							</div>
							{isCardRun(run) ? (
								<div className="mt-1 flex min-w-0 items-center gap-2">
									<Badge variant="neutral">Exécution</Badge>
									{run.repository ? (
										<span className="truncate font-mono text-xs text-muted-foreground">
											{run.repository}
										</span>
									) : null}
								</div>
							) : null}
						</TableCell>
						<TableCell>
							<StatusBadge
								tone={STATUS_TONES[run.status]}
								label={STATUS_LABELS[run.status]}
							/>
						</TableCell>
						<TableCell className="whitespace-nowrap text-muted-foreground figures">
							{dateFormat.format(new Date(run.created_at))}
						</TableCell>
						<TableCell className="hidden whitespace-nowrap font-mono text-xs text-muted-foreground 2xl:table-cell">
							{run.pipeline.join(' → ')}
						</TableCell>
					</TableRow>
				))}
			</TableBody>
		</Table>
	)
}
