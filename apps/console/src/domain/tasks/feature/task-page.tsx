import { Link } from '@tanstack/react-router'
import {
	ArrowUp,
	ClipboardList,
	Layers,
	PanelRight,
	RotateCcw,
} from 'lucide-react'
import {
	type ReactNode,
	useCallback,
	useEffect,
	useMemo,
	useState,
} from 'react'
import { createPortal } from 'react-dom'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { Button } from '#/components/ui/button'
import { Skeleton } from '#/components/ui/skeleton'
import { hueOf } from '#/domain/agent-team/agent-identity'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { TaskArtifactsPanel } from '#/domain/artifacts/feature/task-artifacts-panel'
import { useMe } from '#/domain/identity/hooks/use-me'
import { RunAgentDialog } from '#/domain/tasks/feature/run-agent-dialog'
import { RunStatusPanel } from '#/domain/tasks/feature/run-status-panel'
import { SubtasksPanel } from '#/domain/tasks/feature/subtasks-panel'
import { useAgentOf } from '#/domain/tasks/feature/use-agent-of'
import {
	type AgentRunOutcome,
	handoffNotice,
	takeHandoff,
} from '#/domain/tasks/handoff'
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
import { StatusIcon } from '#/domain/tasks/ui/status-icon'
import {
	type Speaker,
	Thread,
	ThreadMessage,
} from '#/domain/tasks/ui/task-thread'
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

export function TaskPage({ pathId }: Props) {
	const id = taskPathId(pathId)
	const detail = useCard(id)
	const cards = useCards()
	const move = useMoveCard()
	const update = useUpdateCard()
	const team = useTeamAgents().data?.items
	const me = useMe().data
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

	const adopt = useCallback(
		(outcome: AgentRunOutcome | null | undefined) => {
			if (outcome?.run_id) {
				setRunId(outcome.run_id)
				setRunActive(true)
			}
			const notice = handoffNotice(outcome)
			if (notice) notify(notice)
		},
		[notify],
	)

	useEffect(() => {
		adopt(takeHandoff(id))
	}, [id, adopt])

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
	const assignee = team?.find((a) => a.id === task.agentId)
	const repositoryGrants = assignee?.repository_grants ?? []

	const send = (event: React.FormEvent) => {
		event.preventDefault()
		const body = draft.trim()
		if (!body) return
		comment.mutate(
			{ path: { card_id: task.id }, body: { body } },
			{
				onSuccess: (posted) => {
					setDraft('')
					adopt(posted.agent_run)
				},
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
				onSuccess: (patched) => {
					notify(
						agentId
							? `Assignée à ${agentOf(agentId).name}`
							: 'Tâche désassignée',
					)
					adopt(patched.agent_run)
				},
				onError: conflictOr("Le changement d'assignation a échoué"),
			},
		)

	return (
		<>
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
					<div className="flex-1 overflow-auto">
						<div className="mx-auto grid w-full max-w-[820px] gap-6 px-5 pt-8 pb-6 sm:px-8">
							<header className="grid gap-2">
								<div className="flex items-start gap-3">
									<StatusIcon
										status={task.status}
										className="mt-[7px] size-[18px]"
									/>
									<h1 className="m-0 min-w-0 flex-1 text-[22px] leading-snug font-semibold tracking-[-0.01em] text-balance">
										{task.title}
									</h1>
									<span className="mt-1.5 shrink-0 font-mono text-[13px] text-muted-foreground">
										{task.shortId}
									</span>
									{runnable ? (
										<Button
											variant="outline"
											size="sm"
											onClick={() => setDialogOpen(true)}
											disabled={running}
											className="shrink-0"
										>
											<RotateCcw aria-hidden="true" className="size-3.5" />
											Relancer…
										</Button>
									) : null}
								</div>
								<div className="flex flex-wrap items-center gap-x-4 gap-y-1 pl-[30px] text-[13px] text-muted-foreground">
									<span>{STATUS_LABEL[task.status]}</span>
									{task.card.project ? (
										<span>{task.card.project.name}</span>
									) : null}
									{task.agentId ? (
										<span className="flex items-center gap-1.5">
											<AgentAvatar hue={agent.hue} size={16} />
											{agent.name}
										</span>
									) : null}
								</div>
							</header>
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
							<SubtasksPanel task={task} tasks={tasks} onNotify={notify} />
							<TaskArtifactsPanel cardId={task.id} />
							<Thread>
								{task.note ? (
									<ThreadMessage
										speaker={speakerOf(
											{
												author_subject: task.card.created_by_subject,
												author_actor: task.card.created_by_agent
													? `agent:${task.card.created_by_agent}`
													: null,
											},
											team,
											me,
											agentOf,
										)}
										body={task.note}
										at={task.card.created_at}
										plain
									/>
								) : null}
								{comments.map((c) => (
									<ThreadMessage
										key={c.id}
										speaker={speakerOf(c, team, me, agentOf)}
										body={c.body}
										at={c.created_at}
									/>
								))}
							</Thread>
							{shownRun ? (
								<RunStatusPanel
									key={shownRun}
									runId={shownRun}
									agentName={agent.name}
									agentHue={agent.hue}
									onActiveChange={setRunActive}
								/>
							) : null}
						</div>
					</div>
					<div className="mx-auto w-full max-w-[820px] px-5 pb-5 sm:px-8">
						<form
							onSubmit={send}
							className="rounded-2xl border bg-card px-4 pt-3 pb-2.5 shadow-sm focus-within:border-[color-mix(in_srgb,var(--primary)_55%,var(--border))]"
						>
							<textarea
								aria-label="Message"
								placeholder={
									task.agentId
										? `Répondre à ${agent.name}… il reprend avec tout le fil`
										: 'Écrire un commentaire…'
								}
								value={draft}
								rows={2}
								onChange={(event) => setDraft(event.target.value)}
								onKeyDown={(event) => {
									if (
										event.key === 'Enter' &&
										(event.metaKey || event.ctrlKey)
									) {
										send(event)
									}
								}}
								className="block max-h-60 min-h-12 w-full resize-none border-0 bg-transparent p-0 text-[14.5px] outline-none field-sizing-content placeholder:text-muted-foreground"
							/>
							<div className="mt-2 flex flex-wrap items-center gap-2 text-[13px] text-muted-foreground">
								{task.agentId ? (
									<span className="flex min-w-0 items-center gap-1.5">
										<AgentAvatar hue={agent.hue} size={18} />
										<span className="text-foreground">{agent.name}</span>
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
									title="Envoyer (⌘ Entrée)"
									disabled={comment.isPending}
									className="ml-auto grid size-8 place-items-center rounded-full bg-foreground text-background transition-opacity disabled:opacity-40"
								>
									<ArrowUp aria-hidden="true" className="size-4" />
								</button>
							</div>
						</form>
					</div>
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
			{runnable && task.agentId ? (
				<RunAgentDialog
					key={`${task.agentId}:${repositoryGrants.join(',')}`}
					cardId={task.id}
					version={task.version}
					agentId={task.agentId}
					agentName={agent.name}
					repositoryGrants={repositoryGrants}
					hasConnectors={(assignee?.connector_grants.length ?? 0) > 0}
					taskText={`${task.title}\n${task.note ?? ''}`}
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

function speakerOf(
	comment: { author_actor?: string | null; author_subject: string },
	team: ReadonlyArray<{ id: string; slug: string }> | undefined,
	me:
		| {
				subject: string
				display_name?: string | null
				username?: string | null
		  }
		| undefined,
	agentOf: (id: string | null) => { name: string; hue: number },
): Speaker {
	const actor = comment.author_actor
	if (actor?.startsWith('agent:')) {
		const slug = actor.slice('agent:'.length)
		const member = team?.find((a) => a.slug === slug)
		if (member) {
			const shown = agentOf(member.id)
			return { kind: 'agent', name: shown.name, hue: shown.hue }
		}
		return { kind: 'agent', name: slug, hue: hueOf(slug) }
	}
	if (actor) return { kind: 'human', name: actor, mine: false }
	if (me && comment.author_subject === me.subject) {
		return {
			kind: 'human',
			name: me.display_name || me.username || 'Moi',
			mine: true,
		}
	}
	return { kind: 'human', name: 'Membre', mine: false }
}
