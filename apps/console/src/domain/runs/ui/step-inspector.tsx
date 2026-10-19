import { Tabs } from 'radix-ui'
import type { ReactNode } from 'react'
import type { Schemas } from '#/api/api.client'
import { parseExecuteOutput } from '#/domain/runs/execute-output'
import { formatCost, formatLatency, formatTokens } from '#/domain/runs/format'
import { agentName, stepKindLabel } from '#/domain/runs/step-labels'
import { stepOutputValue } from '#/domain/runs/step-output'
import { toolSummaryText } from '#/domain/runs/step-text'
import { ExecuteOutcome } from '#/domain/runs/ui/execute-outcome'
import { JsonView } from '#/domain/runs/ui/json-view'
import {
	STEP_STATUS_LABELS,
	STEP_STATUS_TEXT,
	StepStatusIcon,
} from '#/domain/runs/ui/step-visual'
import { cn } from '#/lib/utils'

const dateTimeFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'short',
	timeStyle: 'medium',
})

const TAB_TRIGGER =
	'-mb-px border-b-2 border-transparent px-3 py-2 text-[13px] font-medium text-muted-foreground transition-colors hover:text-foreground data-[state=active]:border-foreground data-[state=active]:text-foreground focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-ring'

function dateTime(iso: string | null | undefined): string {
	if (!iso) return '—'
	const date = new Date(iso)
	return Number.isNaN(date.getTime()) ? '—' : dateTimeFormat.format(date)
}

interface Props {
	step: Schemas.StepResponse | undefined
	copied: boolean
	onCopy: (text: string) => void
}

function Metadata({ step }: { step: Schemas.StepResponse }) {
	const rows: Array<[string, ReactNode]> = [
		['Agent', step.agent ? agentName(step.agent) : '—'],
		['Modèle', step.model ?? '—'],
		['Tokens entrée', formatTokens(step.input_tokens)],
		['Tokens sortie', formatTokens(step.output_tokens)],
		['Coût', formatCost(step.cost_micros)],
		['Latence', step.latency_ms == null ? '—' : formatLatency(step.latency_ms)],
		['Début', dateTime(step.started_at)],
		['Fin', dateTime(step.finished_at)],
	]
	return (
		<dl className="grid grid-cols-[max-content_minmax(0,1fr)] gap-x-6 gap-y-2 text-[13px]">
			{rows.map(([label, value]) => (
				<div key={label} className="contents">
					<dt className="text-muted-foreground">{label}</dt>
					<dd className="font-mono text-xs break-words figures">{value}</dd>
				</div>
			))}
		</dl>
	)
}

function Tools({ step }: { step: Schemas.StepResponse }) {
	if (step.tool_calls.length === 0) {
		return (
			<p className="text-[13px] text-muted-foreground">Aucun appel d'outil.</p>
		)
	}
	return (
		<ul className="flex flex-col divide-y rounded-md border">
			{step.tool_calls.map((call, index) => (
				<li
					// biome-ignore lint/suspicious/noArrayIndexKey: tool calls have no stable id
					key={index}
					className="flex flex-wrap items-center gap-x-3 gap-y-1 px-3 py-2 text-[13px]"
				>
					<span
						className={cn(
							'text-xs font-medium',
							call.succeeded ? 'text-signal-success' : 'text-signal-danger',
						)}
					>
						{call.succeeded ? 'Réussi' : 'Échoué'}
					</span>
					<span className="font-mono text-xs">{call.tool}</span>
					<span className="text-xs text-muted-foreground figures">
						{formatLatency(call.latency_ms)}
					</span>
					<span className="min-w-0 text-muted-foreground">
						{toolSummaryText(call.summary)}
					</span>
				</li>
			))}
		</ul>
	)
}

function Output({
	step,
	copied,
	onCopy,
}: {
	step: Schemas.StepResponse
	copied: boolean
	onCopy: (text: string) => void
}) {
	const value = stepOutputValue(step)
	const outcome =
		step.kind === 'execute' ? parseExecuteOutput(step.execution) : undefined
	return (
		<div className="flex flex-col gap-3">
			{step.error ? (
				<p role="alert" className="text-[13px] font-medium text-signal-danger">
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
			{value === undefined ? (
				<p className="text-[13px] text-muted-foreground">
					Cette étape n'a rien produit.
				</p>
			) : (
				<JsonView value={value} copied={copied} onCopy={onCopy} />
			)}
		</div>
	)
}

export function StepInspector({ step, copied, onCopy }: Props) {
	if (!step) {
		return (
			<p className="px-4 py-6 text-[13px] text-muted-foreground">
				Aucune étape à inspecter.
			</p>
		)
	}
	return (
		<div className="flex flex-col">
			<div className="flex flex-wrap items-center gap-x-3 gap-y-1 border-b px-4 py-3">
				<StepStatusIcon
					status={step.status}
					className={STEP_STATUS_TEXT[step.status]}
				/>
				<h3 className="text-[15px] font-semibold">
					{stepKindLabel(step.kind)}
				</h3>
				<span
					className={cn('text-xs font-medium', STEP_STATUS_TEXT[step.status])}
				>
					{STEP_STATUS_LABELS[step.status]}
				</span>
			</div>
			<Tabs.Root defaultValue="output" className="flex flex-col">
				<Tabs.List
					aria-label="Détails de l'étape"
					className="flex border-b px-2"
				>
					<Tabs.Trigger value="output" className={TAB_TRIGGER}>
						Sortie
					</Tabs.Trigger>
					<Tabs.Trigger value="tools" className={TAB_TRIGGER}>
						Outils
					</Tabs.Trigger>
					<Tabs.Trigger value="metadata" className={TAB_TRIGGER}>
						Métadonnées
					</Tabs.Trigger>
				</Tabs.List>
				<Tabs.Content value="output" className="p-4">
					<Output step={step} copied={copied} onCopy={onCopy} />
				</Tabs.Content>
				<Tabs.Content value="tools" className="p-4">
					<Tools step={step} />
				</Tabs.Content>
				<Tabs.Content value="metadata" className="p-4">
					<Metadata step={step} />
				</Tabs.Content>
			</Tabs.Root>
		</div>
	)
}
