import { Link } from '@tanstack/react-router'
import { ClipboardList, Layers, PanelRight, Send } from 'lucide-react'
import { type ReactNode, useEffect, useMemo, useState } from 'react'
import { createPortal } from 'react-dom'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { Button } from '#/components/ui/button'
import { Skeleton } from '#/components/ui/skeleton'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { TaskArtifactsPanel } from '#/domain/artifacts/feature/task-artifacts-panel'
import { relativeAge } from '#/domain/dashboard/format'
import { RunAgentDialog } from '#/domain/tasks/feature/run-agent-dialog'
import { RunStatusPanel } from '#/domain/tasks/feature/run-status-panel'
import { SubtasksPanel } from '#/domain/tasks/feature/subtasks-panel'
import { useAgentOf } from '#/domain/tasks/feature/use-agent-of'
import {
	useCard,
	useCards,
	useCommentCard,
	useMoveCard,
	useUpdateCard,
} from '#/domain/tasks/hooks/use-tasks'
import {
	childrenOf,
	columnOfStatus,
	type TaskStatus,
	toTask,
} from '#/domain/tasks/model'
import { canRunAgent, isActiveRun } from '#/domain/tasks/run-agent'
import { taskPathId } from '#/domain/tasks/task-ids'
import { STATUS_LABEL } from '#/domain/tasks/task-list'
import { AgentAvatar } from '#/domain/tasks/ui/agent-chip'
import { PropertiesPanel } from '#/domain/tasks/ui/properties-panel'
import { Toast } from '#/domain/tasks/ui/toast'
import { cn } from '#/lib/utils'

interface Props {
	pathId: string
}

const ACCENT_BANNER =
	'border-[color-mix(in_srgb,var(--primary)_35%,var(--border))] bg-[color-mix(in_srgb,var(--primary)_9%,var(--card))]'

function Banner({
	icon: Icon,
	title,
	accent = false,
	plainTitle = false,
	action,
	children,
}: {
	icon: typeof Layers
	title: string
	accent?: boolean
	plainTitle?: boolean
	action?: ReactNode
	children: ReactNode
}) {
	return (
		<div
			className={cn(
				'flex items-center gap-2.5 rounded-lg border px-3 py-2',
				accent ? ACCENT_BANNER : 'bg-card',
			)}
		>
			<Icon aria-hidden="true" className="size-4 shrink-0" />
			<div className="min-w-0 flex-1">
				<b
					className={cn(
						'block font-semibold',
						plainTitle
							? 'text-foreground'
							: 'text-[color-mix(in_srgb,var(--primary)_80%,var(--foreground))]',
					)}
				>
					{title}
				</b>
				<span className="text-muted-foreground">{children}</span>
			</div>
			{action}
		</div>
	)
}

function useTopbarTarget(): HTMLElement | null {
	const [target, setTarget] = useState<HTMLElement | null>(null)
	useEffect(() => {
		setTarget(document.getElementById('topbar-actions'))
	}, [])
	return target
}

function NotFound() {
	return (
		<Page>
			<EmptyState
				icon={ClipboardList}
				title="Tâche introuvable"
				description="Cette tâche n'existe pas ou a été supprimée."
				action={
					<Button asChild variant="outline">
						<Link to="/tasks" search={{ vue: 'liste', q: '' }}>
							Retour aux tâches
						</Link>
					</Button>
				}
			/>
		</Page>
	)
}

const BUBBLE =
	'max-w-[min(560px,90%)] self-end rounded-[12px_12px_3px_12px] bg-primary px-3 py-[9px] text-primary-foreground'

export function TaskPage({ pathId }: Props) {
	const id = taskPathId(pathId)
	const detail = useCard(id)
	const cards = useCards()
	const move = useMoveCard()
	const update = useUpdateCard()
	const team = useTeamAgents().data?.items
	const comment = useCommentCard()
	const agentOf = useAgentOf()
	const [message, notify] = useNotice()
	const [draft, setDraft] = useState('')
	const [showProps, setShowProps] = useState(true)
	const [runId, setRunId] = useState<string | null>(null)
	const [runActive, setRunActive] = useState(false)
	const [dialogOpen, setDialogOpen] = useState(false)
	const topbar = useTopbarTarget()
	const tasks = useMemo(
		() => (cards.data?.items ?? []).map((card) => toTask(card)),
		[cards.data],
	)

	if (detail.isPending) {
		return (
			<Page>
				<output className="grid gap-2">
					<span className="sr-only">Chargement…</span>
					<Skeleton className="h-8 w-2/3 rounded-md" />
					<Skeleton className="h-24 rounded-md" />
				</output>
			</Page>
		)
	}
	if (detail.isError) {
		const status = (detail.error as { status?: number } | null)?.status
		if (status === 404 || status === 422) return <NotFound />
		return (
			<Page>
				<InlineAlert tone="danger">Impossible de charger la tâche.</InlineAlert>
			</Page>
		)
	}

	const task = toTask(detail.data.card)
	const comments = detail.data.comments
	const agent = agentOf(task.agentId)
	const parent = task.parentId
		? tasks.find((t) => t.id === task.parentId)
		: undefined
	const kids = childrenOf(tasks, task.id)
	const loadedRun = detail.data.runs.find((r) => isActiveRun(r.status))
	const shownRun = runId ?? loadedRun?.id ?? null
	const running = runActive || Boolean(loadedRun)
	const runnable = canRunAgent(task.card.column, task.agentId !== null)

	const send = (event: React.FormEvent) => {
		event.preventDefault()
		const body = draft.trim()
		if (!body) return
		comment.mutate(
			{ path: { card_id: task.id }, body: { body } },
			{
				onSuccess: () => setDraft(''),
				onError: () => notify("Le commentaire n'a pas pu être envoyé"),
			},
		)
	}

	const conflictOr = (fallback: string) => (error: unknown) =>
		notify(
			(error as { status?: number } | null)?.status === 409
				? 'Cette tâche a changé entre-temps, réessaie'
				: fallback,
		)

	const changeStatus = (status: TaskStatus) =>
		move.mutate(
			{
				path: { card_id: task.id },
				body: { to: columnOfStatus(status), expected_version: task.version },
			},
			{
				onSuccess: () => notify(`Statut : ${STATUS_LABEL[status]}`),
				onError: conflictOr('Le changement de statut a échoué'),
			},
		)

	const changeAssignee = (agentId: string | null) =>
		update.mutate(
			{
				path: { card_id: task.id },
				body: { assignee_agent_id: agentId, expected_version: task.version },
			},
			{
				onSuccess: () =>
					notify(
						agentId
							? `Assignée à ${agentOf(agentId).name}`
							: 'Tâche désassignée',
					),
				onError: conflictOr("Le changement d'assignation a échoué"),
			},
		)

	return (
		<>
			<h1 className="sr-only">
				{task.shortId} {task.title}
			</h1>
			{topbar
				? createPortal(
						<button
							type="button"
							aria-label={
								showProps ? 'Masquer les propriétés' : 'Afficher les propriétés'
							}
							onClick={() => setShowProps(!showProps)}
							className="grid size-7 place-items-center rounded-md text-muted-foreground hover:bg-secondary hover:text-foreground"
						>
							<PanelRight aria-hidden="true" className="size-4" />
						</button>,
						topbar,
					)
				: null}
			<div
				className={cn(
					'grid min-h-0 flex-1 leading-[1.4]',
					showProps
						? 'grid-cols-[minmax(0,1fr)_320px]'
						: 'grid-cols-[minmax(0,1fr)]',
				)}
			>
				<section className="flex min-h-0 min-w-0 flex-col">
					<div className="flex flex-1 flex-col gap-3.5 overflow-auto px-4 py-3.5">
						{task.parentId ? (
							<Banner
								icon={Layers}
								title={`Déléguée par ${parent ? agentOf(parent.agentId).name : 'un agent'}`}
							>
								Sous-tâche de{' '}
								<Link
									to="/tasks/$taskId"
									params={{ taskId: taskPathId(task.parentId) }}
									className="text-primary"
								>
									{parent?.shortId ?? task.parentId.slice(0, 8)}
								</Link>
								. Le résultat remonte à l'agent parent.
							</Banner>
						) : null}
						<div className={BUBBLE}>
							<p className="m-0 mb-1.5 font-medium last:mb-0">{task.title}</p>
							{task.note ? (
								<p className="m-0 mb-1.5 whitespace-pre-wrap last:mb-0">
									{task.note}
								</p>
							) : null}
							<small className="mt-1 block text-right text-[0.83em] opacity-75">
								{relativeAge(task.card.created_at)}
							</small>
						</div>
						<SubtasksPanel task={task} tasks={tasks} onNotify={notify} />
						<TaskArtifactsPanel cardId={task.id} />
						{comments.length ? (
							<ul aria-label="Commentaires" className="contents">
								{comments.map((c) => (
									<li key={c.id} className={BUBBLE}>
										<small className="mb-0.5 block text-[0.83em] opacity-75">
											{c.author_actor ?? c.author_subject}
										</small>
										<p className="m-0 whitespace-pre-wrap">{c.body}</p>
										<small className="mt-1 block text-right text-[0.83em] opacity-75">
											{relativeAge(c.created_at)}
										</small>
									</li>
								))}
							</ul>
						) : null}
					</div>
					{shownRun ? (
						<div className="mx-4 mb-2.5">
							<RunStatusPanel
								key={shownRun}
								runId={shownRun}
								onActiveChange={setRunActive}
							/>
						</div>
					) : null}
					{runnable ? (
						<div className="mx-4 mb-2.5 flex">
							<Button onClick={() => setDialogOpen(true)} disabled={running}>
								Exécuter avec {agent.name}
							</Button>
						</div>
					) : null}
					<form
						onSubmit={send}
						className="mx-4 mb-3.5 rounded-[10px] border bg-card px-2.5 py-2 focus-within:border-primary"
					>
						<textarea
							aria-label="Message"
							placeholder="Écrire un commentaire…"
							value={draft}
							onChange={(event) => setDraft(event.target.value)}
							className="block min-h-10 w-full resize-none p-0.5 border-0 bg-transparent outline-none placeholder:text-muted-foreground"
						/>
						<div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
							{task.agentId ? (
								<span className="flex min-w-0 items-center gap-1.5">
									<AgentAvatar hue={agent.hue} size={16} />
									{agent.name}
								</span>
							) : null}
							{agent.model ? (
								<>
									<span className="text-faint">·</span>
									<span>{agent.model}</span>
								</>
							) : null}
							<button
								type="submit"
								aria-label="Envoyer"
								disabled={comment.isPending}
								className="ml-auto grid size-7 place-items-center rounded-full bg-foreground text-background"
							>
								<Send aria-hidden="true" className="size-4" />
							</button>
						</div>
					</form>
				</section>
				{showProps ? (
					<PropertiesPanel
						task={task}
						agent={agent}
						agents={(team ?? []).map((a) => ({
							id: a.id,
							name: agentOf(a.id).name,
						}))}
						subtasks={kids}
						busy={move.isPending}
						assigning={update.isPending}
						onStatus={changeStatus}
						onAssign={changeAssignee}
						onClose={() => setShowProps(false)}
					/>
				) : null}
			</div>
			{runnable ? (
				<RunAgentDialog
					cardId={task.id}
					version={task.version}
					agentName={agent.name}
					open={dialogOpen}
					onOpenChange={setDialogOpen}
					onStarted={(id) => {
						setRunId(id)
						setRunActive(true)
					}}
				/>
			) : null}
			<Toast message={message} />
		</>
	)
}
