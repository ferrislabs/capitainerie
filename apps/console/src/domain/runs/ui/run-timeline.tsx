import {
	Check,
	CircleDashed,
	Loader,
	ShieldCheck,
	ShieldX,
	X,
} from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { SectionCard } from '#/components/layout/section-card'
import { parseExecuteOutput } from '#/domain/runs/execute-output'
import { formatCost, formatLatency, formatTokens } from '#/domain/runs/format'
import { stepDetailText, toolSummaryText } from '#/domain/runs/step-text'
import { ExecuteOutcome } from '#/domain/runs/ui/execute-outcome'
import { cn } from '#/lib/utils'

export type TimelineStep = Schemas.StepResponse

type StepStatus = Schemas.StepStatusResponse

const KIND_LABELS: Record<string, string> = {
	extract: 'Extraction',
	deduplicate: 'Dédoublonnage',
	write: 'Rédaction',
	execute: 'Exécution',
}

const STEP_STATUS_LABELS: Record<StepStatus, string> = {
	running: 'En cours',
	succeeded: 'Terminé',
	failed: 'Échec',
	skipped: 'Ignoré',
}

const STEP_STATUS_TEXT: Record<StepStatus, string> = {
	running: 'text-signal-running',
	succeeded: 'text-signal-success',
	failed: 'text-signal-danger',
	skipped: 'text-muted-foreground',
}

const STEP_BAR: Record<StepStatus, string> = {
	running: 'bg-signal-running',
	succeeded: 'bg-signal-success',
	failed: 'bg-signal-danger',
	skipped: 'bg-muted-foreground/40',
}

const AGENT_PREFIX = 'capitainerie-agent-'

const clockFormat = new Intl.DateTimeFormat('fr-FR', {
	hour: '2-digit',
	minute: '2-digit',
	second: '2-digit',
})

function clock(iso: string | null | undefined): string {
	if (!iso) return '—'
	const date = new Date(iso)
	return Number.isNaN(date.getTime()) ? '—' : clockFormat.format(date)
}

function agentName(agent: string): string {
	return agent.startsWith(AGENT_PREFIX)
		? agent.slice(AGENT_PREFIX.length)
		: agent
}

function StatusIcon({ status }: { status: StepStatus }) {
	const className = cn('size-4', STEP_STATUS_TEXT[status])
	switch (status) {
		case 'running':
			return (
				<Loader
					aria-hidden
					className={cn(className, 'motion-safe:animate-spin')}
				/>
			)
		case 'succeeded':
			return <Check aria-hidden className={className} />
		case 'failed':
			return <X aria-hidden className={className} />
		case 'skipped':
			return <CircleDashed aria-hidden className={className} />
	}
}

/**
 * Une étape se lit comme une ligne d'annuaire des marées : heure à gauche,
 * hauteur (la latence, sur une échelle commune au run) au centre, durée
 * alignée à droite.
 */
function StepItem({ step, scaleMs }: { step: TimelineStep; scaleMs: number }) {
	const outcome =
		step.kind === 'execute' ? parseExecuteOutput(step.execution) : undefined
	const meta = [
		step.agent ? agentName(step.agent) : null,
		step.model ?? null,
		`${formatTokens(step.input_tokens)} → ${formatTokens(step.output_tokens)} tokens`,
		formatCost(step.cost_micros),
	].filter((part): part is string => part !== null)
	const share =
		step.latency_ms == null || scaleMs <= 0
			? 0
			: Math.max(2, (step.latency_ms / scaleMs) * 100)

	return (
		<li className="grid grid-cols-[4.25rem_1.25rem_minmax(0,1fr)_4.5rem] gap-x-3 border-b px-4 py-3 last:border-b-0 sm:grid-cols-[4.25rem_1.25rem_minmax(0,1fr)_minmax(6rem,12rem)_4.5rem]">
			<span className="pt-px font-mono text-xs text-muted-foreground figures">
				{clock(step.started_at)}
			</span>
			<span className="pt-px">
				<StatusIcon status={step.status} />
			</span>
			<div className="flex min-w-0 flex-col gap-1">
				<div className="flex flex-wrap items-baseline gap-x-2">
					<span className="font-semibold">
						{KIND_LABELS[step.kind] ?? step.kind}
					</span>
					<span
						className={cn('text-xs font-medium', STEP_STATUS_TEXT[step.status])}
					>
						{STEP_STATUS_LABELS[step.status]}
					</span>
				</div>
				<p className="text-xs break-words text-muted-foreground figures">
					{meta.join(' · ')}
				</p>
				{step.detail ? (
					<p className="text-[13px]">{stepDetailText(step.detail)}</p>
				) : null}
				{step.error ? (
					<p
						role="alert"
						className="text-[13px] font-medium text-signal-danger"
					>
						{step.error}
					</p>
				) : null}
				{outcome ? (
					<ExecuteOutcome
						output={outcome}
						failed={step.status === 'failed'}
						stepError={step.error}
					/>
				) : null}
				{step.tool_calls.length > 0 ? (
					<ul className="mt-1 flex flex-col gap-1 border-l border-border pl-3">
						{step.tool_calls.map((call, index) => (
							<li
								// biome-ignore lint/suspicious/noArrayIndexKey: tool calls have no stable id
								key={index}
								className="flex flex-wrap items-center gap-x-2 text-[13px]"
							>
								{call.succeeded ? (
									<Check
										aria-label="Réussi"
										className="size-3.5 text-signal-success"
									/>
								) : (
									<X
										aria-label="Échoué"
										className="size-3.5 text-signal-danger"
									/>
								)}
								<span className="font-mono text-xs">{call.tool}</span>
								<span className="text-xs text-muted-foreground figures">
									{formatLatency(call.latency_ms)}
								</span>
								<span className="text-muted-foreground">
									{toolSummaryText(call.summary)}
								</span>
							</li>
						))}
					</ul>
				) : null}
			</div>
			<div className="hidden pt-1.5 sm:block" aria-hidden="true">
				<div className="h-1.5 w-full rounded-full bg-muted">
					<div
						className={cn('h-full rounded-full', STEP_BAR[step.status])}
						style={{ width: `${share}%` }}
					/>
				</div>
			</div>
			<span className="text-right text-[13px] font-medium figures">
				{step.latency_ms == null ? '' : formatLatency(step.latency_ms)}
			</span>
		</li>
	)
}

function DecisionItem({ decision }: { decision: Schemas.DecisionResponse }) {
	const Icon = decision.allowed ? ShieldCheck : ShieldX
	const verdict = decision.allowed ? 'autorisé' : 'refusé'
	const parts = [decision.action, decision.actor].filter(
		(part): part is string => typeof part === 'string' && part !== '',
	)
	const reason = decision.reason ? ` (${decision.reason})` : ''

	return (
		<li className="grid grid-cols-[4.25rem_1.25rem_minmax(0,1fr)] gap-x-3 border-b px-4 py-2 last:border-b-0">
			<span className="font-mono text-xs text-muted-foreground figures">
				{clock(decision.decided_at)}
			</span>
			<Icon
				aria-hidden
				className={cn(
					'size-4',
					decision.allowed ? 'text-signal-success' : 'text-signal-danger',
				)}
			/>
			<span
				className={cn(
					'text-[13px] break-words',
					!decision.allowed && 'font-medium text-signal-danger',
				)}
			>{`${parts.join(' — ')} — ${verdict}${reason}`}</span>
		</li>
	)
}

interface Props {
	steps: ReadonlyArray<TimelineStep>
	decisions: ReadonlyArray<Schemas.DecisionResponse>
}

export function RunTimeline({ steps, decisions }: Props) {
	const ordered = [...steps].sort((a, b) => a.seq - b.seq)
	const scaleMs = ordered.reduce(
		(max, step) => Math.max(max, step.latency_ms ?? 0),
		0,
	)
	const refused = decisions.filter((decision) => !decision.allowed).length

	return (
		<div className="grid items-start gap-5 2xl:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
			<SectionCard
				title="Étapes"
				description="Heure de début, latence sur l'échelle du run."
				flush
			>
				{ordered.length === 0 ? (
					<p className="px-4 py-6 text-[13px] text-muted-foreground">
						Aucune étape.
					</p>
				) : (
					<ol>
						{ordered.map((step) => (
							<StepItem key={step.seq} step={step} scaleMs={scaleMs} />
						))}
					</ol>
				)}
			</SectionCard>
			<SectionCard
				title="Décisions"
				description={
					decisions.length === 0
						? 'Contrôles d’autorisation des écritures des agents.'
						: `${decisions.length} contrôle${decisions.length > 1 ? 's' : ''}, ${refused} refus.`
				}
				flush
			>
				{decisions.length === 0 ? (
					<p className="px-4 py-6 text-[13px] text-muted-foreground">
						Aucune décision.
					</p>
				) : (
					<ul>
						{decisions.map((decision) => (
							<DecisionItem
								key={`${decision.decided_at}-${decision.action}-${decision.resource_id ?? ''}`}
								decision={decision}
							/>
						))}
					</ul>
				)}
			</SectionCard>
		</div>
	)
}
