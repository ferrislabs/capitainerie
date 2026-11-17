import { Link } from '@tanstack/react-router'
import {
	ClipboardList,
	Clock,
	Copy,
	Layers,
	PanelRight,
	Paperclip,
	Radio,
	Send,
	ThumbsUp,
} from 'lucide-react'
import { type ReactNode, useEffect, useState } from 'react'
import { createPortal } from 'react-dom'
import { DemoBanner } from '#/components/layout/demo-banner'
import { EmptyState } from '#/components/layout/empty-state'
import { useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { childTasks, updateTask, useDemoState } from '#/domain/demo'
import { SubtasksPanel } from '#/domain/tasks/feature/subtasks-panel'
import { useAgentOf } from '#/domain/tasks/feature/use-agent-of'
import { addComment, useComments } from '#/domain/tasks/mock/comments-store'
import { parseTaskId, taskPathId } from '#/domain/tasks/task-ids'
import { STATUS_LABEL } from '#/domain/tasks/task-list'
import { AgentAvatar } from '#/domain/tasks/ui/agent-chip'
import { PropertiesPanel } from '#/domain/tasks/ui/properties-panel'
import { Sev } from '#/domain/tasks/ui/sev'
import { StatusIcon } from '#/domain/tasks/ui/status-icon'
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

const ICON_BUTTON =
	'grid size-6 place-items-center rounded-md text-faint hover:bg-secondary hover:text-foreground'

export function TaskPage({ pathId }: Props) {
	const id = parseTaskId(pathId)
	const state = useDemoState((s) => s)
	const task = state.tasks.find((t) => t.id === id)
	const agentOf = useAgentOf()
	const comments = useComments(id)
	const [message, notify] = useNotice()
	const [draft, setDraft] = useState('')
	const [showProps, setShowProps] = useState(true)
	const topbar = useTopbarTarget()

	if (!task) {
		return (
			<Page>
				<DemoBanner />
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

	const agent = agentOf(task.agentId)
	const parent = task.parentId
		? state.tasks.find((t) => t.id === task.parentId)
		: undefined
	const kids = childTasks(state, task.id)
	const sourceName = task.sourceId
		? (state.sources.find((s) => s.id === task.sourceId)?.name ?? task.sourceId)
		: null
	const first = task.id === 'NAT-611'
	const check = () => notify('Vérification immédiate lancée')

	const send = (event: React.FormEvent) => {
		event.preventDefault()
		const text = draft.trim()
		if (!text) return
		addComment(id, { author: 'Toi', text })
		setDraft('')
	}

	const verify = (
		<Button variant="outline" onClick={check}>
			Vérifier maintenant
		</Button>
	)

	return (
		<>
			<DemoBanner />
			<h1 className="sr-only">
				{task.id} {task.title}
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
						{first ? (
							<Banner
								icon={Clock}
								accent
								title="L'agent reprend dans 2 h 27"
								action={verify}
							>
								Aujourd'hui 23:00 (ton fuseau) · tentative 1
							</Banner>
						) : null}
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
									{task.parentId}
								</Link>
								. Le résultat remonte à l'agent parent.
							</Banner>
						) : null}
						{task.sourceId ? (
							<Banner icon={Radio} title="Déclenchée par une source">
								<Link
									to="/sources/$sourceId/{-$tab}"
									params={{ sourceId: task.sourceId }}
									className="text-primary"
								>
									{sourceName}
								</Link>
								{task.sourceValue ? (
									<>
										{' '}
										· valeur reçue{' '}
										<span className="font-mono text-[11.5px]">
											{task.sourceValue}
										</span>
									</>
								) : null}
							</Banner>
						) : null}
						<div className="max-w-[min(560px,90%)] self-end rounded-[12px_12px_3px_12px] bg-primary px-3 py-[9px] text-primary-foreground">
							<p className="m-0 mb-1.5 last:mb-0">
								{first
									? "Surfaces publiques + chaîne d'appro de l'appliance on-prem + licence ed25519 + RLS Postgres. Findings sensibles → issue confidentielle ; correctifs → MR jamais mergée. Aucun secret en clair."
									: task.title}
							</p>
							{first ? (
								<p className="m-0 mb-1.5 last:mb-0">
									Coupe-circuit : échecs ≥ 3 sur 7 j ⇒ pause
								</p>
							) : null}
							<small className="mt-1 block text-right text-[0.83em] opacity-75">
								07:00
							</small>
						</div>
						<div>
							<div className="mb-1 flex items-center gap-[7px] font-semibold">
								<AgentAvatar hue={agent.hue} size={22} />
								{agent.fullName}
							</div>
							<div>
								{first
									? "L'agent ACP a signalé un échec de limite de terminal."
									: task.status === 'done'
										? 'Pièce traitée et archivée sous compta/2026/11. Aucun écart détecté.'
										: "Prise en charge. Je reviens avec un état d'avancement."}
							</div>
							<div className="mt-1 flex items-center gap-1 text-[11.5px] text-faint">
								<span>07:00</span>
								<span className="flex-1" />
								<button
									type="button"
									aria-label="Copier"
									onClick={() => notify('Copié')}
									className={ICON_BUTTON}
								>
									<Copy aria-hidden="true" className="size-4" />
								</button>
								<button
									type="button"
									aria-label="Utile"
									onClick={() => notify('Merci pour le retour')}
									className={ICON_BUTTON}
								>
									<ThumbsUp aria-hidden="true" className="size-4" />
								</button>
							</div>
						</div>
						<SubtasksPanel task={task} onNotify={notify} />
						{task.dossier ? (
							<SectionCard
								title="Dossier légal"
								flush
								className="my-2"
								actions={<Sev tone="ok">2 indices concordants</Sev>}
							>
								{task.dossier.map(([label, value]) => (
									<div
										key={label}
										className="grid min-w-0 grid-cols-[180px_minmax(0,1fr)] items-center gap-2.5 border-t px-3 py-2 first:border-t-0"
									>
										<span className="text-muted-foreground">{label}</span>
										<span>{value}</span>
									</div>
								))}
							</SectionCard>
						) : null}
						{task.status === 'progress' ? (
							<>
								<div className="flex items-center gap-2 text-muted-foreground">
									<StatusIcon status="progress" />
									<b className="font-medium text-foreground">En cours</b>
									<span className="text-faint">depuis 85 h 32</span>
								</div>
								<Banner
									icon={Clock}
									plainTitle
									title="Reprise dans 2 h 27"
									action={verify}
								>
									Répondre réveille l'agent tout de suite, avant la vérification
									planifiée.
								</Banner>
							</>
						) : null}
						{comments.length ? (
							<ul aria-label="Commentaires" className="contents">
								{comments.map((c) => (
									<li
										key={c.id}
										className="max-w-[min(560px,90%)] self-end rounded-[12px_12px_3px_12px] bg-primary px-3 py-[9px] text-primary-foreground"
									>
										<p className="m-0">{c.text}</p>
										<small className="mt-1 block text-right text-[0.83em] opacity-75">
											à l'instant
										</small>
									</li>
								))}
							</ul>
						) : null}
					</div>
					<form
						onSubmit={send}
						className="mx-4 mb-3.5 rounded-[10px] border bg-card px-2.5 py-2 focus-within:border-primary"
					>
						<textarea
							aria-label="Message"
							placeholder={`Écrire à ${agent.name} : décris ce que tu veux faire…`}
							value={draft}
							onChange={(event) => setDraft(event.target.value)}
							className="block min-h-10 w-full resize-none p-0.5 border-0 bg-transparent outline-none placeholder:text-muted-foreground"
						/>
						<div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
							<button
								type="button"
								aria-label="Joindre"
								onClick={() => notify('Joindre un fichier ou un artifact')}
								className={ICON_BUTTON.replace(
									'text-faint',
									'text-muted-foreground',
								)}
							>
								<Paperclip aria-hidden="true" className="size-4" />
							</button>
							<span className="flex min-w-0 items-center gap-1.5">
								<AgentAvatar hue={agent.hue} size={16} />
								{agent.name}
							</span>
							<span className="text-faint">·</span>
							<span>{agent.model}</span>
							<button
								type="submit"
								aria-label="Envoyer"
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
						subtasks={kids}
						onStatus={(status) => {
							updateTask(task.id, { status })
							notify(`Statut : ${STATUS_LABEL[status]}`)
						}}
						onClose={() => setShowProps(false)}
					/>
				) : null}
			</div>
			<Toast message={message} />
		</>
	)
}
