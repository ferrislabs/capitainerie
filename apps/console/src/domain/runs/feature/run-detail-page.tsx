import { Link } from '@tanstack/react-router'
import { ArrowLeft, ExternalLink } from 'lucide-react'
import { useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { StatusBadge } from '#/components/layout/status-badge'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import { Skeleton } from '#/components/ui/skeleton'
import {
	formatCost,
	formatLatency,
	formatTokens,
	runDurationMs,
} from '#/domain/runs/format'
import { useRunTimeline } from '#/domain/runs/hooks/use-run-timeline'
import { isCardRun, runTitle } from '#/domain/runs/run-title'
import { STATUS_LABELS, STATUS_TONES } from '#/domain/runs/status'
import { defaultSelectedSeq } from '#/domain/runs/step-selection'
import { RunTimeline } from '#/domain/runs/ui/run-timeline'
import { useCopyToClipboard } from '#/hooks/use-copy-to-clipboard'
import { useNow } from '#/hooks/use-now'
import { ApiRequestError } from '#/lib/api-error'
import { getRuntimeConfig } from '#/lib/runtime-config'

const NOT_FOUND_STATUSES = new Set([400, 404, 422])

const dateFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'short',
	timeStyle: 'short',
})

function BackLink() {
	return (
		<Link
			to="/runs"
			className="inline-flex w-fit items-center gap-1 text-[13px] text-muted-foreground hover:text-foreground"
		>
			<ArrowLeft aria-hidden className="size-3.5" />
			Runs
		</Link>
	)
}

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
		<dl className="grid grid-cols-2 gap-px overflow-hidden rounded-lg border bg-border sm:grid-cols-4">
			{items.map(([label, value]) => (
				<div key={label} className="flex flex-col gap-0.5 bg-card px-4 py-3">
					<dt className="text-xs text-muted-foreground">{label}</dt>
					<dd className="text-lg leading-tight font-semibold figures">
						{value}
					</dd>
				</div>
			))}
		</dl>
	)
}

function DetailSkeleton() {
	return (
		<output className="flex flex-col gap-5">
			<span className="sr-only">Chargement…</span>
			<div className="flex flex-col gap-3 border-b pb-5">
				<Skeleton className="h-3 w-12" />
				<Skeleton className="h-6 w-80 max-w-full" />
				<Skeleton className="h-3 w-56" />
			</div>
			<Skeleton className="h-16 w-full rounded-lg" />
			<Skeleton className="h-64 w-full rounded-lg" />
		</output>
	)
}

export function RunDetailPage({ runId }: { runId: string }) {
	const timeline = useRunTimeline(runId)
	const [picked, setPicked] = useState<number | undefined>()
	const { copied, copy, reset } = useCopyToClipboard()
	const runStatus = timeline.data?.run.status
	const active = runStatus === 'queued' || runStatus === 'running'
	const nowMs = useNow(active)

	if (timeline.isError) {
		const notFound =
			timeline.error instanceof ApiRequestError &&
			NOT_FOUND_STATUSES.has(timeline.error.status)
		return (
			<Page>
				<BackLink />
				<InlineAlert tone="danger">
					{notFound
						? 'Run introuvable. Il a peut-être été supprimé, ou le lien est incorrect.'
						: 'Impossible de charger le run. Vérifiez votre connexion puis réessayez.'}
				</InlineAlert>
				<div className="flex gap-2">
					{notFound ? null : (
						<Button size="sm" onClick={() => void timeline.refetch()}>
							Réessayer
						</Button>
					)}
					<Button asChild size="sm" variant="outline">
						<Link to="/runs">Retour aux runs</Link>
					</Button>
				</div>
			</Page>
		)
	}
	if (timeline.isPending) {
		return (
			<Page>
				<DetailSkeleton />
			</Page>
		)
	}

	const { run, steps, decisions, totals } = timeline.data
	const duration = runDurationMs(run)
	const selectedSeq = steps.some((step) => step.seq === picked)
		? picked
		: defaultSelectedSeq(steps)
	const stepLatency = steps.reduce((sum, s) => sum + (s.latency_ms ?? 0), 0)
	const tracesUrl = getRuntimeConfig().tracesUrl

	return (
		<Page>
			<PageHeader
				leading={<BackLink />}
				title={runTitle(run)}
				meta={
					<>
						<StatusBadge
							tone={STATUS_TONES[run.status]}
							label={STATUS_LABELS[run.status]}
						/>
						{isCardRun(run) ? <Badge variant="neutral">Exécution</Badge> : null}
					</>
				}
				description={
					<span className="figures">
						Lancé le {dateFormat.format(new Date(run.created_at))}
						{duration === null ? '' : ` · Durée ${formatLatency(duration)}`}
						<span className="ml-2 font-mono text-[11px]">{run.id}</span>
						{run.repository ? (
							<span className="ml-2 font-mono text-[11px]">
								{run.repository}
							</span>
						) : null}
						{run.card_id ? (
							<Link
								to="/kanban"
								hash={`card-${run.card_id}`}
								className="ml-2 text-link hover:underline"
							>
								Voir la carte
							</Link>
						) : null}
					</span>
				}
				actions={
					run.trace_id ? (
						<Button asChild variant="outline" size="sm">
							<a
								href={`${tracesUrl}/trace/${run.trace_id}`}
								target="_blank"
								rel="noreferrer"
							>
								Voir la trace
								<ExternalLink aria-hidden className="size-3.5" />
							</a>
						</Button>
					) : null
				}
			/>
			{run.failure ? (
				<InlineAlert tone="danger">{run.failure}</InlineAlert>
			) : null}
			<TotalsBar totals={totals} latencyMs={duration ?? stepLatency} />
			<RunTimeline
				run={run}
				steps={steps}
				decisions={decisions}
				nowMs={nowMs}
				selectedSeq={selectedSeq}
				onSelect={(seq) => {
					reset()
					setPicked(seq)
				}}
				copied={copied}
				onCopy={copy}
			/>
		</Page>
	)
}
