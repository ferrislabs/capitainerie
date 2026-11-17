import { useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { Button } from '#/components/ui/button'
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from '#/components/ui/dialog'
import { NativeSelect } from '#/components/ui/native-select'
import { TextInput } from '#/components/ui/text-input'
import { Textarea } from '#/components/ui/textarea'
import {
	addTask,
	DEMO_PROJECTS,
	demoStore,
	nextTaskId,
	shortName,
	useDemoState,
} from '#/domain/demo'
import { taskPathId } from '#/domain/tasks/task-ids'

interface Props {
	open: boolean
	onOpenChange: (open: boolean) => void
}

const NONE = 'Aucun'

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
			<DialogContent>
				<form
					onSubmit={submit}
					aria-label="Nouvelle tâche"
					className="flex flex-col gap-3"
				>
					<DialogHeader>
						<DialogTitle>Nouvelle tâche</DialogTitle>
						<DialogDescription>
							Décris le travail et assigne-le à un agent.
						</DialogDescription>
					</DialogHeader>
					<label htmlFor="nt-title" className="flex flex-col gap-1 text-[13px]">
						Titre
						<TextInput
							id="nt-title"
							value={title}
							onChange={(event) => setTitle(event.target.value)}
						/>
					</label>
					{error ? (
						<p role="alert" className="text-[13px] text-signal-danger">
							{error}
						</p>
					) : null}
					<label htmlFor="nt-desc" className="flex flex-col gap-1 text-[13px]">
						Description
						<Textarea
							id="nt-desc"
							value={description}
							onChange={(event) => setDescription(event.target.value)}
						/>
					</label>
					<div className="grid grid-cols-2 gap-3">
						<label
							htmlFor="nt-agent"
							className="flex flex-col gap-1 text-[13px]"
						>
							Agent
							<NativeSelect
								id="nt-agent"
								value={agentId || (agents[0]?.id ?? '')}
								onChange={(event) => setAgentId(event.target.value)}
							>
								{agents.map((a) => (
									<option key={a.id} value={a.id}>
										{shortName(a.name)}
									</option>
								))}
							</NativeSelect>
						</label>
						<label
							htmlFor="nt-project"
							className="flex flex-col gap-1 text-[13px]"
						>
							Projet
							<NativeSelect
								id="nt-project"
								value={project}
								onChange={(event) => setProject(event.target.value)}
							>
								<option value={NONE}>{NONE}</option>
								{DEMO_PROJECTS.map((p) => (
									<option key={p.name} value={p.name}>
										{p.name}
									</option>
								))}
							</NativeSelect>
						</label>
					</div>
					<DialogFooter>
						<Button type="button" variant="outline" onClick={close}>
							Annuler
						</Button>
						<Button type="submit">Créer et assigner</Button>
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	)
}
