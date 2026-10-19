import { ShieldCheck, ShieldX } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { SectionCard } from '#/components/layout/section-card'
import { moveSelection } from '#/domain/runs/step-selection'
import { computeAxis, effectiveNow } from '#/domain/runs/timeline-axis'
import { RunFrise } from '#/domain/runs/ui/run-frise'
import { StepInspector } from '#/domain/runs/ui/step-inspector'
import { StepList, stepOptionId } from '#/domain/runs/ui/step-list'
import { cn } from '#/lib/utils'

export type TimelineStep = Schemas.StepResponse

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
	run: Schemas.RunResponse
	steps: ReadonlyArray<TimelineStep>
	decisions: ReadonlyArray<Schemas.DecisionResponse>
	nowMs: number
	selectedSeq: number | undefined
	onSelect: (seq: number) => void
	copied: boolean
	onCopy: (text: string) => void
}

export function RunTimeline({
	run,
	steps,
	decisions,
	nowMs,
	selectedSeq,
	onSelect,
	copied,
	onCopy,
}: Props) {
	const ordered = [...steps].sort((a, b) => a.seq - b.seq)
	const active =
		(run.status === 'queued' || run.status === 'running') && !run.finished_at
	const clockMs = effectiveNow(run, ordered, nowMs, active)
	const axis = computeAxis(run, ordered, nowMs, active)
	const selected = ordered.find((step) => step.seq === selectedSeq)
	const refused = decisions.filter((decision) => !decision.allowed).length

	function handleKey(key: string): boolean {
		const next = moveSelection(
			ordered.map((step) => step.seq),
			selectedSeq,
			key,
		)
		if (next === undefined) return false
		onSelect(next)
		document.getElementById(stepOptionId(next))?.focus()
		return true
	}

	return (
		<div className="flex flex-col gap-5">
			<SectionCard
				title="Frise"
				description="Chaque étape sur l'axe du temps du run."
				flush
			>
				{ordered.length === 0 ? (
					<p className="px-4 py-6 text-[13px] text-muted-foreground">
						Aucune étape.
					</p>
				) : (
					<RunFrise
						steps={ordered}
						axis={axis}
						nowMs={clockMs}
						selectedSeq={selectedSeq}
						onSelect={onSelect}
					/>
				)}
			</SectionCard>
			{ordered.length === 0 ? null : (
				<div className="grid items-start gap-5 lg:grid-cols-[minmax(14rem,20rem)_minmax(0,1fr)]">
					<SectionCard
						title="Étapes"
						description="Flèches haut et bas pour changer d'étape."
						flush
					>
						<StepList
							steps={ordered}
							selectedSeq={selectedSeq}
							onSelect={onSelect}
							onKeyDown={handleKey}
						/>
					</SectionCard>
					<SectionCard title="Inspecteur" flush>
						<StepInspector step={selected} copied={copied} onCopy={onCopy} />
					</SectionCard>
				</div>
			)}
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
