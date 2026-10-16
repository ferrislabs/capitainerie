import {
	Check,
	CircleDashed,
	Loader,
	ShieldCheck,
	ShieldX,
	X,
} from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { formatCost, formatLatency, formatTokens } from '#/domain/runs/format'

type StepStatus = Schemas.StepStatusResponse

const KIND_LABELS: Record<string, string> = {
	extract: 'Extraction',
	deduplicate: 'Dédoublonnage',
	write: 'Rédaction',
}

const STEP_STATUS_LABELS: Record<StepStatus, string> = {
	running: 'En cours',
	succeeded: 'Terminé',
	failed: 'Échec',
	skipped: 'Ignoré',
}

const AGENT_PREFIX = 'capitainerie-agent-'

function agentName(agent: string): string {
	return agent.startsWith(AGENT_PREFIX)
		? agent.slice(AGENT_PREFIX.length)
		: agent
}

function StatusIcon({ status }: { status: StepStatus }) {
	switch (status) {
		case 'running':
			return <Loader aria-hidden className="size-4 animate-spin text-primary" />
		case 'succeeded':
			return <Check aria-hidden className="size-4 text-green-600" />
		case 'failed':
			return <X aria-hidden className="size-4 text-destructive" />
		case 'skipped':
			return (
				<CircleDashed aria-hidden className="size-4 text-muted-foreground" />
			)
	}
}

function StepItem({ step }: { step: Schemas.StepResponse }) {
	const meta = [
		step.agent ? agentName(step.agent) : null,
		step.model ?? null,
		`${formatTokens(step.input_tokens)} → ${formatTokens(step.output_tokens)} tokens`,
		step.latency_ms == null ? null : formatLatency(step.latency_ms),
		formatCost(step.cost_micros),
	].filter((part): part is string => part !== null)

	return (
		<li className="flex flex-col gap-1 rounded-md border p-3">
			<div className="flex items-center gap-2">
				<StatusIcon status={step.status} />
				<span className="font-medium">
					{KIND_LABELS[step.kind] ?? step.kind}
				</span>
				<span className="text-sm text-muted-foreground">
					{STEP_STATUS_LABELS[step.status]}
				</span>
			</div>
			<p className="text-xs text-muted-foreground">{meta.join(' · ')}</p>
			{step.detail ? <p className="text-sm">{step.detail}</p> : null}
			{step.error ? (
				<p role="alert" className="text-sm text-destructive">
					{step.error}
				</p>
			) : null}
			{step.tool_calls.length > 0 ? (
				<ul className="mt-1 flex flex-col gap-1 border-l pl-3">
					{step.tool_calls.map((call, index) => (
						<li
							// biome-ignore lint/suspicious/noArrayIndexKey: tool calls have no stable id
							key={index}
							className="flex items-center gap-2 text-sm"
						>
							{call.succeeded ? (
								<Check
									aria-label="Réussi"
									className="size-3.5 text-green-600"
								/>
							) : (
								<X aria-label="Échoué" className="size-3.5 text-destructive" />
							)}
							<span className="font-mono">{call.tool}</span>
							<span className="text-xs text-muted-foreground">
								{formatLatency(call.latency_ms)}
							</span>
							<span className="text-muted-foreground">{call.summary}</span>
						</li>
					))}
				</ul>
			) : null}
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
		<li
			className={`flex items-center gap-2 text-sm ${decision.allowed ? 'text-green-700 dark:text-green-400' : 'text-destructive'}`}
		>
			<Icon aria-hidden className="size-4" />
			<span>{`${parts.join(' — ')} — ${verdict}${reason}`}</span>
		</li>
	)
}

interface Props {
	steps: ReadonlyArray<Schemas.StepResponse>
	decisions: ReadonlyArray<Schemas.DecisionResponse>
}

export function RunTimeline({ steps, decisions }: Props) {
	const ordered = [...steps].sort((a, b) => a.seq - b.seq)
	return (
		<div className="flex flex-col gap-6">
			<section className="flex flex-col gap-2">
				<h2 className="text-lg font-semibold">Étapes</h2>
				{ordered.length === 0 ? (
					<p className="text-sm text-muted-foreground">Aucune étape.</p>
				) : (
					<ol className="flex flex-col gap-2">
						{ordered.map((step) => (
							<StepItem key={step.seq} step={step} />
						))}
					</ol>
				)}
			</section>
			<section className="flex flex-col gap-2">
				<h2 className="text-lg font-semibold">Décisions</h2>
				{decisions.length === 0 ? (
					<p className="text-sm text-muted-foreground">Aucune décision.</p>
				) : (
					<ul className="flex flex-col gap-1">
						{decisions.map((decision) => (
							<DecisionItem
								key={`${decision.decided_at}-${decision.action}-${decision.resource_id ?? ''}`}
								decision={decision}
							/>
						))}
					</ul>
				)}
			</section>
		</div>
	)
}
