import { Link } from '@tanstack/react-router'
import { ClipboardList, Layers, PanelRight, Radio, Send } from 'lucide-react'
import { useState } from 'react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { EmptyState } from '#/components/layout/empty-state'
import { Notice, useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { Textarea } from '#/components/ui/textarea'
import { childTasks, updateTask, useDemoState } from '#/domain/demo'
import { SubtasksPanel } from '#/domain/tasks/feature/subtasks-panel'
import { useAgentOf } from '#/domain/tasks/feature/use-agent-of'
import { addComment, useComments } from '#/domain/tasks/mock/comments-store'
import { parseTaskId, taskPathId } from '#/domain/tasks/task-ids'
import { STATUS_LABEL } from '#/domain/tasks/task-list'
import { AgentChip } from '#/domain/tasks/ui/agent-chip'
import { PropertiesPanel } from '#/domain/tasks/ui/properties-panel'

interface Props {
	pathId: string
}

function Banner({
	icon: Icon,
	title,
	children,
}: {
	icon: typeof Layers
	title: string
	children: React.ReactNode
}) {
	return (
		<div className="flex items-start gap-3 rounded-lg border bg-card px-3 py-2.5 text-[13px]">
			<Icon
				aria-hidden="true"
				className="mt-0.5 size-4 text-muted-foreground"
			/>
			<div className="flex flex-col gap-0.5">
				<b>{title}</b>
				<span className="text-muted-foreground">{children}</span>
			</div>
		</div>
	)
}

export function TaskPage({ pathId }: Props) {
	const id = parseTaskId(pathId)
	const state = useDemoState((s) => s)
	const task = state.tasks.find((t) => t.id === id)
	const agentOf = useAgentOf()
	const comments = useComments(id)
	const [message, notify] = useNotice()
	const [draft, setDraft] = useState('')
	const [showProps, setShowProps] = useState(true)

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

	const send = (event: React.FormEvent) => {
		event.preventDefault()
		const text = draft.trim()
		if (!text) return
		addComment(id, { author: 'Toi', text })
		setDraft('')
	}

	return (
		<Page>
			<DemoBanner />
			<PageHeader
				leading={
					<Link
						to="/tasks"
						search={{ vue: 'liste', q: '' }}
						className="text-[13px] text-muted-foreground hover:underline"
					>
						Retour aux tâches
					</Link>
				}
				title={
					<>
						<span className="font-mono text-muted-foreground">{task.id}</span>{' '}
						{task.title}
					</>
				}
				description={STATUS_LABEL[task.status]}
				actions={
					showProps ? undefined : (
						<Button variant="outline" onClick={() => setShowProps(true)}>
							<PanelRight aria-hidden="true" />
							Afficher les propriétés
						</Button>
					)
				}
			/>
			<Notice message={message} />
			<div
				className={
					showProps
						? 'grid gap-4 lg:grid-cols-[minmax(0,1fr)_18rem]'
						: 'grid gap-4'
				}
			>
				<div className="flex min-w-0 flex-col gap-3">
					{task.sourceId ? (
						<Banner icon={Radio} title="Déclenchée par une source">
							<Link
								to="/sources/$sourceId/{-$tab}"
								params={{ sourceId: task.sourceId }}
								className="text-signal-running hover:underline"
							>
								{sourceName}
							</Link>
							{task.sourceValue ? (
								<>
									{' '}
									· valeur reçue{' '}
									<span className="font-mono">{task.sourceValue}</span>
								</>
							) : null}
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
								className="text-signal-running hover:underline"
							>
								{task.parentId}
							</Link>
							. Le résultat remonte à l'agent parent.
						</Banner>
					) : null}
					<div className="rounded-lg border bg-card px-3 py-2.5 text-[13px]">
						{task.title}
					</div>
					<div className="flex flex-col gap-1 rounded-lg px-3 py-2 text-[13px]">
						<AgentChip name={agent.name} hue={agent.hue} size={22} />
						<p>
							{task.status === 'done'
								? 'Tâche traitée. Aucun écart détecté.'
								: "Prise en charge. Je reviens avec un état d'avancement."}
						</p>
					</div>
					<SubtasksPanel task={task} onNotify={notify} />
					{task.dossier ? (
						<SectionCard title="Dossier légal">
							<dl className="grid grid-cols-[10rem_1fr] gap-x-3 gap-y-1 text-[13px]">
								{task.dossier.map(([label, value]) => (
									<div key={label} className="contents">
										<dt className="text-muted-foreground">{label}</dt>
										<dd>{value}</dd>
									</div>
								))}
							</dl>
						</SectionCard>
					) : null}
					{comments.length ? (
						<ul aria-label="Commentaires" className="flex flex-col gap-2">
							{comments.map((c) => (
								<li
									key={c.id}
									className="rounded-lg border bg-card px-3 py-2 text-[13px]"
								>
									<b>{c.author}</b>
									<p>{c.text}</p>
								</li>
							))}
						</ul>
					) : null}
					<form onSubmit={send} className="flex flex-col gap-2">
						<Textarea
							aria-label="Message"
							placeholder={`Écrire à ${agent.name} : décris ce que tu veux faire…`}
							value={draft}
							onChange={(event) => setDraft(event.target.value)}
						/>
						<div className="flex justify-end">
							<Button type="submit" aria-label="Envoyer">
								<Send aria-hidden="true" />
								Envoyer
							</Button>
						</div>
					</form>
				</div>
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
		</Page>
	)
}
