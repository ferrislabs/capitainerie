import { Link } from '@tanstack/react-router'
import { ArrowLeft, ExternalLink } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { Badge } from '#/components/ui/badge'
import {
	formatCost,
	formatLatency,
	formatTokens,
	runDurationMs,
} from '#/domain/runs/format'
import { useRunTimeline } from '#/domain/runs/hooks/use-run-timeline'
import { STATUS_LABELS, STATUS_VARIANTS } from '#/domain/runs/status'
import { RunTimeline } from '#/domain/runs/ui/run-timeline'
import { ApiRequestError } from '#/lib/api-error'
import { getRuntimeConfig } from '#/lib/runtime-config'

const dateFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'short',
	timeStyle: 'short',
})

function TotalsBar({
	totals,
	latencyMs,
}: {
	totals: Schemas.UsageTotals
	latencyMs: number
}) {
	const items = [
		['Tokens entrée', formatTokens(totals.input_tokens)],
		['Tokens sortie', formatTokens(totals.output_tokens)],
		['Coût', formatCost(totals.cost_micros)],
		['Latence totale', formatLatency(latencyMs)],
	]
	return (
		<dl className="grid grid-cols-2 gap-4 rounded-md border p-4 sm:grid-cols-4">
			{items.map(([label, value]) => (
				<div key={label}>
					<dt className="text-xs text-muted-foreground">{label}</dt>
					<dd className="text-lg font-semibold">{value}</dd>
				</div>
			))}
		</dl>
	)
}

export function RunDetailPage({ runId }: { runId: string }) {
	const timeline = useRunTimeline(runId)

	if (timeline.isError) {
		const notFound =
			timeline.error instanceof ApiRequestError && timeline.error.status === 404
		return (
			<p role="alert" className="text-sm text-destructive">
				{notFound ? 'Run introuvable.' : 'Impossible de charger le run.'}
			</p>
		)
	}
	if (timeline.isPending) {
		return <p className="text-sm text-muted-foreground">Chargement…</p>
	}

	const { run, steps, decisions, totals } = timeline.data
	const duration = runDurationMs(run)
	const stepLatency = steps.reduce((sum, s) => sum + (s.latency_ms ?? 0), 0)
	const tracesUrl = getRuntimeConfig().tracesUrl

	return (
		<section className="flex flex-col gap-6">
			<Link
				to="/runs"
				className="inline-flex w-fit items-center gap-1 text-sm text-muted-foreground hover:underline"
			>
				<ArrowLeft aria-hidden className="size-4" />
				Runs
			</Link>
			<header className="flex flex-col gap-2">
				<div className="flex flex-wrap items-center gap-3">
					<h1 className="text-2xl font-semibold">{run.transcript_title}</h1>
					<Badge variant={STATUS_VARIANTS[run.status]}>
						{STATUS_LABELS[run.status]}
					</Badge>
				</div>
				<p className="text-sm text-muted-foreground">
					Lancé le {dateFormat.format(new Date(run.created_at))}
					{duration === null ? '' : ` · Durée ${formatLatency(duration)}`}
				</p>
				{run.trace_id ? (
					<a
						href={`${tracesUrl}/trace/${run.trace_id}`}
						target="_blank"
						rel="noreferrer"
						className="inline-flex w-fit items-center gap-1 text-sm underline"
					>
						Voir la trace
						<ExternalLink aria-hidden className="size-3.5" />
					</a>
				) : null}
				{run.failure ? (
					<p role="alert" className="text-sm text-destructive">
						{run.failure}
					</p>
				) : null}
			</header>
			<TotalsBar totals={totals} latencyMs={duration ?? stepLatency} />
			<RunTimeline steps={steps} decisions={decisions} />
		</section>
	)
}
