import { Link } from '@tanstack/react-router'
import { X } from 'lucide-react'
import type { ReactNode } from 'react'
import {
	PRIORITY_LABEL,
	type Task,
	type TaskStatus,
} from '#/domain/tasks/model'
import { taskPathId } from '#/domain/tasks/task-ids'
import { STATUS_LABEL, STATUS_ORDER } from '#/domain/tasks/task-list'
import { AgentChip } from '#/domain/tasks/ui/agent-chip'
import type { AgentView } from '#/domain/tasks/ui/tasks-view'

interface Props {
	task: Task
	agent: AgentView
	subtasks: ReadonlyArray<Task>
	busy?: boolean
	onStatus: (status: TaskStatus) => void
	onClose: () => void
}

function Group({ title, children }: { title: string; children: ReactNode }) {
	return (
		<div className="px-3.5 pt-2.5 pb-1">
			<h4 className="m-0 mb-1.5 font-mono text-[10px] font-medium tracking-[.09em] text-faint uppercase">
				{title}
			</h4>
			{children}
		</div>
	)
}

function Row({ label, children }: { label: string; children: ReactNode }) {
	return (
		<div className="grid min-h-7 grid-cols-[100px_minmax(0,1fr)] items-center gap-2">
			<span className="text-muted-foreground">{label}</span>
			<span className="min-w-0 [overflow-wrap:anywhere]">{children}</span>
		</div>
	)
}

const LINK = 'text-xs text-primary'

const dateFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'medium',
	timeStyle: 'short',
})

const formatDate = (iso: string): string => {
	const time = Date.parse(iso)
	return Number.isNaN(time) ? '' : dateFormat.format(time)
}

export function PropertiesPanel({
	task,
	agent,
	subtasks,
	busy = false,
	onStatus,
	onClose,
}: Props) {
	const card = task.card
	return (
		<aside
			aria-label="Propriétés"
			className="min-h-0 overflow-auto border-l bg-card"
		>
			<header className="sticky top-0 flex h-[46px] items-center justify-between border-b bg-card pr-2.5 pl-3.5 font-semibold">
				Propriétés
				<button
					type="button"
					aria-label="Fermer"
					onClick={onClose}
					className="grid size-7 place-items-center rounded-md text-muted-foreground hover:bg-secondary hover:text-foreground"
				>
					<X aria-hidden="true" className="size-4" />
				</button>
			</header>
			<Group title="Travail">
				<Row label="Statut">
					<select
						aria-label="Statut"
						value={task.status}
						disabled={busy}
						onChange={(event) => onStatus(event.target.value as TaskStatus)}
						className="max-w-full border-0 bg-transparent py-0.5"
					>
						{STATUS_ORDER.map((s) => (
							<option key={s} value={s} className="bg-card">
								{STATUS_LABEL[s]}
							</option>
						))}
					</select>
				</Row>
				<Row label="Assigné à">
					{task.agentId ? (
						<Link
							to="/agents/$agentId/{-$tab}"
							params={{ agentId: task.agentId }}
							className="text-foreground"
						>
							<AgentChip
								name={agent.name}
								hue={agent.hue}
								className="text-foreground"
							/>
						</Link>
					) : (
						'Personne'
					)}
				</Row>
				<Row label="Priorité">{PRIORITY_LABEL[card.priority]}</Row>
				<Row label="Projet">{task.project || 'Aucun'}</Row>
				<Row label="Client">{card.client?.name ?? 'Aucun'}</Row>
				<Row label="Étiquettes">
					{card.labels.length ? card.labels.join(', ') : 'Aucune'}
				</Row>
			</Group>
			<Group title="Relations">
				<Row label="Parent">
					{task.parentId ? (
						<Link
							to="/tasks/$taskId"
							params={{ taskId: taskPathId(task.parentId) }}
							className={LINK}
						>
							{task.parentId.slice(0, 8)}
						</Link>
					) : (
						'Aucun'
					)}
				</Row>
				<Row label="Sous-tâches">
					{subtasks.length ? (
						<span className="flex flex-wrap gap-x-1">
							{subtasks.map((c) => (
								<Link
									key={c.id}
									to="/tasks/$taskId"
									params={{ taskId: taskPathId(c.id) }}
									className={LINK}
								>
									{c.shortId}
								</Link>
							))}
						</span>
					) : (
						'Aucune'
					)}
				</Row>
			</Group>
			<Group title="À propos">
				<Row label="Origine">{card.created_by_subject}</Row>
				<Row label="Créée">{formatDate(card.created_at)}</Row>
				<Row label="Mise à jour">{task.age}</Row>
			</Group>
		</aside>
	)
}
