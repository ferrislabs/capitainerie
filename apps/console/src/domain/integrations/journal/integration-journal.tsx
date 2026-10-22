import { ChevronRight } from 'lucide-react'
import { type ReactNode, useId, useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { StatusBadge, type StatusTone } from '#/components/layout/status-badge'
import { Badge } from '#/components/ui/badge'
import { EnvironmentBadge } from '#/domain/integrations/environment-badge'
import {
	reasonLabel,
	targetSummary,
	toolLabel,
} from '#/domain/integrations/journal/action-labels'
import {
	type DiffKind,
	hasChanges,
	lineDiff,
} from '#/domain/integrations/journal/line-diff'
import { RevertControl } from '#/domain/integrations/journal/revert-control'
import { canRevert } from '#/domain/integrations/journal/revert-rules'
import { relativeDate } from '#/domain/kanban/relative-date'
import { prettyJson } from '#/domain/runs/json-tokens'
import { cn } from '#/lib/utils'

type Action = Schemas.IntegrationActionResponse

const OUTCOMES: Record<
	Schemas.ActionOutcomeDto,
	{ label: string; tone: StatusTone }
> = {
	pending: { label: 'En cours', tone: 'running' },
	succeeded: { label: 'Réussie', tone: 'success' },
	denied: { label: 'Refusée', tone: 'attention' },
	failed: { label: 'En échec', tone: 'danger' },
}

const DIFF_PREFIXES: Record<DiffKind, string> = {
	same: '  ',
	added: '+ ',
	removed: '- ',
}

const DIFF_CLASSES: Record<DiffKind, string> = {
	same: 'text-muted-foreground',
	added: 'bg-signal-success-soft text-signal-success',
	removed: 'bg-signal-danger-soft text-signal-danger',
}

const dateFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'short',
	timeStyle: 'short',
})

function chronological(actions: ReadonlyArray<Action>): Array<Action> {
	return [...actions].sort(
		(a, b) => Date.parse(a.created_at) - Date.parse(b.created_at),
	)
}

function present(value: unknown): boolean {
	return value !== undefined && value !== null
}

function Panel({ title, children }: { title: string; children: ReactNode }) {
	const titleId = useId()
	return (
		<section aria-labelledby={titleId} className="flex min-w-0 flex-col gap-1">
			<h4 id={titleId} className="text-xs font-medium text-muted-foreground">
				{title}
			</h4>
			{children}
		</section>
	)
}

function JsonPanel({ title, value }: { title: string; value: unknown }) {
	return (
		<Panel title={title}>
			{present(value) ? (
				<pre className="max-h-64 overflow-auto rounded-md border bg-card p-2 font-mono text-xs leading-relaxed">
					<code>{prettyJson(value)}</code>
				</pre>
			) : (
				<p className="text-[13px] text-muted-foreground">—</p>
			)}
		</Panel>
	)
}

function DiffPanel({ before, after }: { before: unknown; after: unknown }) {
	const diff = lineDiff(prettyJson(before), prettyJson(after))
	const seen: Record<DiffKind, number> = { same: 0, added: 0, removed: 0 }
	return (
		<Panel title="Différence">
			{hasChanges(diff) ? (
				<pre className="max-h-64 overflow-auto rounded-md border bg-card py-1 font-mono text-xs leading-relaxed">
					<code>
						{diff.map((line) => (
							<span
								key={`${line.kind}-${seen[line.kind]++}`}
								data-kind={line.kind}
								className={cn(
									'block px-2 whitespace-pre',
									DIFF_CLASSES[line.kind],
								)}
							>
								{DIFF_PREFIXES[line.kind]}
								{line.text}
							</span>
						))}
					</code>
				</pre>
			) : (
				<p className="text-[13px] text-muted-foreground">Aucune différence.</p>
			)}
		</Panel>
	)
}

function restorationLabel(action: Action, byId: ReadonlyMap<string, Action>) {
	if (!action.reverts_action_id) return undefined
	const original = byId.get(action.reverts_action_id)
	return original
		? `Restauration de l’action du ${dateFormat.format(new Date(original.created_at))}`
		: 'Restauration d’une autre action'
}

function JournalRow({
	action,
	byId,
	revert,
}: {
	action: Action
	byId: ReadonlyMap<string, Action>
	revert: RevertOptions | undefined
}) {
	const [open, setOpen] = useState(false)
	const [restored, setRestored] = useState(false)
	const detailId = useId()
	const outcome = OUTCOMES[action.outcome]
	const reason =
		action.outcome === 'denied' || action.outcome === 'failed'
			? reasonLabel(action.reason_code, action.reason)
			: undefined
	const restoration = restorationLabel(action, byId)

	return (
		<li className="flex flex-col">
			<button
				type="button"
				aria-expanded={open}
				aria-controls={open ? detailId : undefined}
				className="flex w-full items-start gap-2 px-3 py-2 text-left transition-colors hover:bg-muted/50 focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-ring"
				onClick={() => setOpen((current) => !current)}
			>
				<ChevronRight
					aria-hidden="true"
					className={cn(
						'mt-0.5 size-4 shrink-0 text-muted-foreground transition-transform',
						open && 'rotate-90',
					)}
				/>
				<span className="flex min-w-0 flex-1 flex-col gap-1">
					<span className="flex flex-wrap items-center gap-x-2 gap-y-1">
						<time
							dateTime={action.created_at}
							title={dateFormat.format(new Date(action.created_at))}
							className="text-xs text-muted-foreground figures"
						>
							{relativeDate(action.created_at)}
						</time>
						<span className="text-[13px] font-medium">
							{action.instance_name}
						</span>
						<EnvironmentBadge environment={action.instance_environment} />
						<StatusBadge tone={outcome.tone} label={outcome.label} />
						{action.reverted_by_action_id ? (
							<Badge variant="muted">Restaurée</Badge>
						) : null}
					</span>
					<span className="flex flex-wrap items-baseline gap-x-2 text-[13px]">
						<span>{toolLabel(action.tool)}</span>
						<span className="font-mono text-xs break-all text-muted-foreground">
							{targetSummary(action.target)}
						</span>
					</span>
					{reason ? (
						<span className="text-xs text-muted-foreground">{reason}</span>
					) : null}
					{restoration ? (
						<span className="text-xs text-muted-foreground">{restoration}</span>
					) : null}
				</span>
			</button>
			{open ? (
				<div
					id={detailId}
					className="flex flex-col gap-3 border-t bg-muted/30 px-3 py-3"
				>
					<JsonPanel title="Arguments" value={action.arguments} />
					<JsonPanel title="Avant" value={action.before} />
					<JsonPanel title="Après" value={action.after} />
					{present(action.before) && present(action.after) ? (
						<DiffPanel before={action.before} after={action.after} />
					) : null}
					{restored ? (
						<InlineAlert tone="success">État d’avant restauré.</InlineAlert>
					) : null}
					{revert && canRevert(action) ? (
						<RevertControl
							action={action}
							locked={revert.locked}
							onRestored={() => setRestored(true)}
						/>
					) : null}
				</div>
			) : null}
		</li>
	)
}

export interface RevertOptions {
	locked: boolean
}

interface Props {
	actions: ReadonlyArray<Action>
	revert?: RevertOptions
}

export function IntegrationJournal({ actions, revert }: Props) {
	const ordered = chronological(actions)
	const byId = new Map(ordered.map((action) => [action.id, action]))

	return (
		<ol className="flex flex-col divide-y">
			{ordered.map((action) => (
				<JournalRow
					key={action.id}
					action={action}
					byId={byId}
					revert={revert}
				/>
			))}
		</ol>
	)
}
