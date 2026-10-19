import { Link } from '@tanstack/react-router'
import { Radio } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { EmptyState } from '#/components/layout/empty-state'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import { formatCost, formatElapsed, formatTokens } from '#/domain/runs/format'
import { runTitle } from '#/domain/runs/run-title'
import { agentName, stepKindLabel } from '#/domain/runs/step-labels'

interface Props {
	runs: ReadonlyArray<Schemas.ActiveRunResponse>
	nowMs: number
}

function elapsedMs(run: Schemas.RunResponse, nowMs: number): number {
	return nowMs - Date.parse(run.started_at ?? run.created_at)
}

function stepText(active: Schemas.ActiveRunResponse): string {
	const { current_step: current, steps_done: done, steps_total: total } = active
	if (!current) return 'En file'
	return `${Math.min(done + 1, total)}/${total} ${stepKindLabel(current.kind)}`
}

function subtitle(run: Schemas.RunResponse): string | undefined {
	return run.repository ?? undefined
}

export function ActiveRunsTable({ runs, nowMs }: Props) {
	if (runs.length === 0) {
		return (
			<EmptyState
				icon={Radio}
				title="Aucun agent ne tourne."
				description="Les runs en attente ou en cours apparaîtront ici dès leur lancement."
			/>
		)
	}

	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>Run</TableHead>
					<TableHead>Agent</TableHead>
					<TableHead>Étape</TableHead>
					<TableHead className="text-right">Durée</TableHead>
					<TableHead className="text-right">Tokens</TableHead>
					<TableHead className="text-right">Coût</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{runs.map((active) => {
					const { run, current_step: current } = active
					const running = run.status === 'running'
					const repository = subtitle(run)
					return (
						<TableRow key={run.id} className="relative">
							<TableCell className="max-w-[26rem]">
								<div className="flex min-w-0 items-center gap-2.5">
									<span
										role="img"
										aria-label={running ? 'En cours' : 'En attente'}
										className={
											running
												? 'size-2 shrink-0 rounded-full bg-signal-running motion-safe:animate-signal'
												: 'size-2 shrink-0 rounded-full border border-signal-neutral'
										}
									/>
									<div className="flex min-w-0 flex-col">
										<Link
											to="/runs/$runId"
											params={{ runId: run.id }}
											className="truncate font-medium text-foreground after:absolute after:inset-0 hover:text-link hover:underline focus-visible:outline-2 focus-visible:outline-ring"
										>
											{runTitle(run)}
										</Link>
										{repository ? (
											<span className="truncate font-mono text-xs text-muted-foreground">
												{repository}
											</span>
										) : null}
									</div>
								</div>
							</TableCell>
							<TableCell>
								{current?.agent ? (
									<span className="font-mono text-xs">
										{agentName(current.agent)}
									</span>
								) : (
									<span className="text-muted-foreground">—</span>
								)}
							</TableCell>
							<TableCell className="whitespace-nowrap figures">
								{stepText(active)}
							</TableCell>
							<TableCell className="text-right whitespace-nowrap figures">
								{formatElapsed(elapsedMs(run, nowMs))}
							</TableCell>
							<TableCell className="text-right whitespace-nowrap figures">
								{formatTokens(active.input_tokens)} →{' '}
								{formatTokens(active.output_tokens)}
							</TableCell>
							<TableCell className="text-right whitespace-nowrap figures">
								{formatCost(active.cost_micros)}
							</TableCell>
						</TableRow>
					)
				})}
			</TableBody>
		</Table>
	)
}
