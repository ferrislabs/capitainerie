import { useNavigate } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { Button } from '#/components/ui/button'
import { Dialog, DialogContent, DialogTitle } from '#/components/ui/dialog'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { useProjects } from '#/domain/projects/hooks/use-projects'
import { MAX_SUBTASK_DEPTH } from '#/domain/tasks/delegation'
import { useCards, useCreateCard } from '#/domain/tasks/hooks/use-tasks'
import { ancestorCount, toTask } from '#/domain/tasks/model'
import { shortTaskId, taskPathId } from '#/domain/tasks/task-ids'

interface Props {
	open: boolean
	onOpenChange: (open: boolean) => void
	defaultAgentId?: string
}

const NONE = ''

const PRIORITIES: ReadonlyArray<{
	value: Schemas.PriorityDto
	label: string
}> = [
	{ value: 'low', label: 'Basse' },
	{ value: 'normal', label: 'Normale' },
	{ value: 'high', label: 'Haute' },
	{ value: 'urgent', label: 'Urgente' },
]

const CONTROL =
	'rounded-[6px] border bg-background px-[9px] py-1.5 text-[13px] text-foreground outline-none focus-visible:border-primary'

function createError(error: unknown): string {
	const status = (error as { status?: number } | null)?.status
	if (status === 403) return "Tu n'as pas le droit de créer des tâches."
	if (status === 422) return 'Les informations de la tâche sont invalides.'
	return 'La création a échoué. Réessaie.'
}

export function NewTaskDialog({
	open,
	onOpenChange,
	defaultAgentId = NONE,
}: Props) {
	const agents = useTeamAgents().data?.items ?? []
	const projects = useProjects(false).data?.items ?? []
	const cards = useCards().data?.items
	const create = useCreateCard()
	const navigate = useNavigate()
	const [title, setTitle] = useState('')
	const [description, setDescription] = useState('')
	const [agentId, setAgentId] = useState(defaultAgentId)
	const [projectId, setProjectId] = useState(NONE)
	const [priority, setPriority] = useState<Schemas.PriorityDto>('normal')
	const [parentId, setParentId] = useState(NONE)
	const [error, setError] = useState<string | null>(null)
	const parents = useMemo(() => {
		const tasks = (cards ?? []).map((card) => toTask(card))
		return tasks.filter(
			(t) =>
				t.status !== 'done' &&
				t.status !== 'cancelled' &&
				ancestorCount(tasks, t.id) < MAX_SUBTASK_DEPTH,
		)
	}, [cards])

	const close = () => {
		setTitle('')
		setDescription('')
		setAgentId(defaultAgentId)
		setProjectId(NONE)
		setPriority('normal')
		setParentId(NONE)
		setError(null)
		create.reset()
		onOpenChange(false)
	}

	const submit = (event: React.FormEvent) => {
		event.preventDefault()
		if (!title.trim()) {
			setError('Écris le titre de la tâche')
			return
		}
		setError(null)
		create.mutate(
			{
				body: {
					title: title.trim(),
					description: description.trim() || null,
					priority,
					assignee_agent_id: agentId || null,
					project_id: projectId || null,
					parent_id: parentId || null,
				},
			},
			{
				onSuccess: (card) => {
					close()
					void navigate({
						to: '/tasks/$taskId',
						params: { taskId: taskPathId(card.id) },
					})
				},
			},
		)
	}

	return (
		<Dialog
			open={open}
			onOpenChange={(next) => (next ? onOpenChange(true) : close())}
		>
			<DialogContent
				aria-describedby={undefined}
				className="top-[12vh] max-w-[560px] translate-y-0 gap-0 overflow-hidden rounded-[11px] p-0 shadow-float"
			>
				<form onSubmit={submit} aria-label="Nouvelle tâche">
					<div className="flex h-[46px] items-center border-b px-3.5">
						<DialogTitle className="text-[13px] leading-none font-semibold">
							Nouvelle tâche
						</DialogTitle>
					</div>
					<div className="grid gap-2.5 p-3.5">
						<label
							htmlFor="nt-title"
							className="grid gap-1 text-xs text-muted-foreground"
						>
							Titre
							<input
								id="nt-title"
								placeholder="Ce que l'agent doit accomplir"
								value={title}
								onChange={(event) => setTitle(event.target.value)}
								className={CONTROL}
							/>
						</label>
						{error ? (
							<p role="alert" className="text-[13px] text-signal-danger">
								{error}
							</p>
						) : null}
						<label
							htmlFor="nt-desc"
							className="grid gap-1 text-xs text-muted-foreground"
						>
							Description
							<textarea
								id="nt-desc"
								rows={3}
								placeholder="Contexte, contraintes, définition de terminé"
								value={description}
								onChange={(event) => setDescription(event.target.value)}
								className={CONTROL}
							/>
						</label>
						<div className="grid gap-2.5 sm:grid-cols-2">
							<label
								htmlFor="nt-agent"
								className="grid gap-1 text-xs text-muted-foreground"
							>
								Assigner à
								<select
									id="nt-agent"
									value={agentId}
									onChange={(event) => setAgentId(event.target.value)}
									className={CONTROL}
								>
									{' '}
									<option value={NONE}>Aucun</option>
									{agents.map((a) => (
										<option key={a.id} value={a.id}>
											{a.name}
										</option>
									))}
								</select>
							</label>
							<label
								htmlFor="nt-project"
								className="grid gap-1 text-xs text-muted-foreground"
							>
								Projet
								<select
									id="nt-project"
									value={projectId}
									onChange={(event) => setProjectId(event.target.value)}
									className={CONTROL}
								>
									{' '}
									<option value={NONE}>Aucun</option>{' '}
									{projects.map((p) => (
										<option key={p.project.id} value={p.project.id}>
											{' '}
											{p.project.name}{' '}
										</option>
									))}{' '}
								</select>{' '}
							</label>
							<label
								htmlFor="nt-priority"
								className="grid gap-1 text-xs text-muted-foreground"
							>
								Priorité
								<select
									id="nt-priority"
									value={priority}
									onChange={(event) =>
										setPriority(event.target.value as Schemas.PriorityDto)
									}
									className={CONTROL}
								>
									{PRIORITIES.map((p) => (
										<option key={p.value} value={p.value}>
											{p.label}
										</option>
									))}
								</select>
							</label>
							<label
								htmlFor="nt-parent"
								className="grid gap-1 text-xs text-muted-foreground"
							>
								Tâche parente
								<select
									id="nt-parent"
									value={parentId}
									onChange={(event) => setParentId(event.target.value)}
									className={CONTROL}
								>
									<option value={NONE}>Aucune</option>
									{parents.map((t) => (
										<option key={t.id} value={t.id}>
											{shortTaskId(t.id)} · {t.title}
										</option>
									))}
								</select>
							</label>
						</div>
						{create.isError ? (
							<p role="alert" className="text-[13px] text-signal-danger">
								{createError(create.error)}
							</p>
						) : null}
						<div className="flex justify-end gap-2">
							<Button type="button" variant="outline" onClick={close}>
								Annuler
							</Button>
							<Button type="submit" disabled={create.isPending}>
								Créer la tâche
							</Button>
						</div>
					</div>
				</form>
			</DialogContent>
		</Dialog>
	)
}
