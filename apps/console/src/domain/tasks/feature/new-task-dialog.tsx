import { useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { Button } from '#/components/ui/button'
import { Dialog, DialogContent, DialogTitle } from '#/components/ui/dialog'
import {
	addTask,
	DEMO_PROJECTS,
	demoStore,
	nextTaskId,
	useDemoState,
} from '#/domain/demo'
import { taskPathId } from '#/domain/tasks/task-ids'

interface Props {
	open: boolean
	onOpenChange: (open: boolean) => void
}

const NONE = 'Aucun'

const CONTROL =
	'rounded-[6px] border bg-background px-[9px] py-1.5 text-[13px] text-foreground outline-none focus-visible:border-primary'

export function NewTaskDialog({ open, onOpenChange }: Props) {
	const agents = useDemoState((s) => s.agents)
	const navigate = useNavigate()
	const [title, setTitle] = useState('')
	const [description, setDescription] = useState('')
	const [agentId, setAgentId] = useState('')
	const [project, setProject] = useState(NONE)
	const [error, setError] = useState<string | null>(null)

	const close = () => {
		setTitle('')
		setDescription('')
		setProject(NONE)
		setError(null)
		onOpenChange(false)
	}

	const submit = (event: React.FormEvent) => {
		event.preventDefault()
		if (!title.trim()) {
			setError('Écris le titre de la tâche')
			return
		}
		const id = nextTaskId(demoStore.state)
		addTask({
			id,
			title: title.trim(),
			status: 'todo',
			agentId: agentId || (agents[0]?.id ?? ''),
			age: "à l'instant",
			project: project === NONE ? '' : project,
			note: description.trim() || undefined,
		})
		close()
		void navigate({ to: '/tasks/$taskId', params: { taskId: taskPathId(id) } })
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
									value={agentId || (agents[0]?.id ?? '')}
									onChange={(event) => setAgentId(event.target.value)}
									className={CONTROL}
								>
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
									value={project}
									onChange={(event) => setProject(event.target.value)}
									className={CONTROL}
								>
									<option value={NONE}>{NONE}</option>
									{DEMO_PROJECTS.map((p) => (
										<option key={p.name} value={p.name}>
											{p.name}
										</option>
									))}
								</select>
							</label>
						</div>
						<div className="flex justify-end gap-2">
							<Button type="button" variant="outline" onClick={close}>
								Annuler
							</Button>
							<Button type="submit">Créer la tâche</Button>
						</div>
					</div>
				</form>
			</DialogContent>
		</Dialog>
	)
}
