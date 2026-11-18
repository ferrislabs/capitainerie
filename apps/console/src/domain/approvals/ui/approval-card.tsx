import { Link } from '@tanstack/react-router'
import { useId, useState } from 'react'
import { Button } from '#/components/ui/button'
import {
	type Approval,
	type Decision,
	formatWhen,
	KIND_LABEL,
	prettyJson,
	STATUS_LABEL,
	STATUS_TONE,
	toolCallOf,
} from '#/domain/approvals/model'
import { Sev } from '#/domain/audit/ui/sev'
import { shortTaskId, taskPathId } from '#/domain/tasks/task-ids'

export type { Decision }

interface Props {
	approval: Approval
	requester: string | null
	busy: boolean
	onDecide: (id: string, decision: Decision, note: string) => void
}

const PRE =
	'm-0 max-h-72 overflow-auto rounded-md border bg-background p-2.5 font-mono text-[12px] leading-[1.5] whitespace-pre-wrap break-words'

export function ApprovalCard({
	approval: a,
	requester,
	busy,
	onDecide,
}: Props) {
	const [note, setNote] = useState('')
	const noteId = useId()
	const call = toolCallOf(a)
	const hasDetail = Object.keys(a.detail).length > 0
	const pending = a.status === 'pending'

	return (
		<article
			aria-label={a.title}
			className="grid gap-2.5 rounded-lg border bg-card px-3.5 py-3"
		>
			<header className="flex flex-wrap items-start gap-2">
				<div className="min-w-0 flex-1">
					<h2 className="m-0 text-[14px] leading-[1.4] font-semibold">
						{a.title}
					</h2>
					<p className="m-0 text-xs text-muted-foreground">
						{KIND_LABEL[a.kind]}
						{requester ? ` · demandée par ${requester}` : ''}
						{` · ${formatWhen(a.created_at)}`}
						{a.card_id ? (
							<>
								{' · '}
								<Link
									to="/tasks/$taskId"
									params={{ taskId: taskPathId(a.card_id) }}
									className="text-primary hover:underline"
								>
									Tâche {shortTaskId(a.card_id)}
								</Link>
							</>
						) : null}
					</p>
				</div>
				<Sev tone={STATUS_TONE[a.status]}>{STATUS_LABEL[a.status]}</Sev>
			</header>
			{call ? (
				<div className="grid gap-1.5">
					<p className="m-0 text-xs text-muted-foreground">
						Outil{' '}
						<code className="rounded bg-secondary px-1.5 py-px font-mono text-[12px] text-foreground">
							{call.tool}
						</code>
					</p>
					<section aria-label="Arguments">
						<pre className={PRE}>{prettyJson(call.arguments)}</pre>
					</section>
				</div>
			) : hasDetail ? (
				<section aria-label="Détail">
					<pre className={PRE}>{prettyJson(a.detail)}</pre>
				</section>
			) : null}
			{pending ? (
				<div className="grid gap-2">
					<label
						htmlFor={noteId}
						className="grid gap-1 text-xs text-muted-foreground"
					>
						Note (optionnelle)
						<textarea
							id={noteId}
							rows={2}
							value={note}
							onChange={(event) => setNote(event.target.value)}
							className="rounded-[6px] border bg-background px-[9px] py-1.5 text-[13px] text-foreground outline-none focus-visible:border-primary"
						/>
					</label>
					<div className="flex justify-end gap-2">
						<Button
							type="button"
							size="sm"
							variant="outline"
							disabled={busy}
							aria-label={`Refuser : ${a.title}`}
							onClick={() => onDecide(a.id, 'reject', note)}
						>
							Refuser
						</Button>
						<Button
							type="button"
							size="sm"
							disabled={busy}
							aria-label={`Approuver : ${a.title}`}
							onClick={() => onDecide(a.id, 'approve', note)}
						>
							Approuver
						</Button>
					</div>
				</div>
			) : (
				<p className="m-0 text-xs text-muted-foreground">
					{STATUS_LABEL[a.status]}
					{a.decided_by ? ` par ${a.decided_by}` : ''}
					{a.decided_at ? ` le ${formatWhen(a.decided_at)}` : ''}
					{a.decision_note ? (
						<span className="mt-1 block text-foreground">
							« {a.decision_note} »
						</span>
					) : null}
				</p>
			)}
		</article>
	)
}
