import { Link } from '@tanstack/react-router'
import { X } from 'lucide-react'
import type { ReactNode } from 'react'
import type { DemoTask, DemoTaskStatus } from '#/domain/demo'
import { taskPathId } from '#/domain/tasks/task-ids'
import { STATUS_LABEL, STATUS_ORDER } from '#/domain/tasks/task-list'
import { AgentChip, HumanAvatar } from '#/domain/tasks/ui/agent-chip'
import type { AgentView } from '#/domain/tasks/ui/tasks-view'

interface Props {
	task: DemoTask
	agent: AgentView
	subtasks: ReadonlyArray<DemoTask>
	onStatus: (status: DemoTaskStatus) => void
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

export function PropertiesPanel({
	task,
	agent,
	subtasks,
	onStatus,
	onClose,
}: Props) {
	const first = task.id === 'NAT-611'
	const blockedBy =
		task.note && task.status === 'blocked'
			? task.note.replace('attend la sous-tâche ', '')
			: 'Aucune'
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
						onChange={(event) => onStatus(event.target.value as DemoTaskStatus)}
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
				</Row>
				<Row label="Projet">{task.project || 'Aucun'}</Row>
				<Row label="Étiquettes">{first ? 'routine' : 'Aucune'}</Row>
			</Group>
			<Group title="Relations">
				<Row label="Parent">
					{task.parentId ? (
						<Link
							to="/tasks/$taskId"
							params={{ taskId: taskPathId(task.parentId) }}
							className={LINK}
						>
							{task.parentId}
						</Link>
					) : (
						'Aucun'
					)}
				</Row>
				<Row label="Bloquée par">{blockedBy}</Row>
				<Row label="Bloque">{task.id === 'NAT-680' ? 'NAT-679' : 'Aucune'}</Row>
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
									{c.id}
								</Link>
							))}
						</span>
					) : (
						'Aucune'
					)}
				</Row>
			</Group>
			<Group title="Exécution">
				<Row label="Relecteurs">Aucun</Row>
				<Row label="Approbateurs">Aucun</Row>
				<Row label="Nouvelle tentative">
					{first ? 'Dans 2 h 27 · 17 nov. · tentative 1' : 'Aucune'}
				</Row>
				<Row label="Moniteur">Aucun</Row>
				<Row label="Chien de garde">Aucun</Row>
			</Group>
			<Group title="À propos">
				<Row label="Origine">
					<span className="flex items-center gap-1.5 text-xs">
						<HumanAvatar initials="FG" size={18} />
						François-Guillaume Ribreau
					</span>
				</Row>
				<Row label="Démarrée">14 nov. 2026, 07:00</Row>
				<Row label="Créée">14 nov. 2026, 07:00</Row>
				<Row label="Mise à jour">il y a 3 j</Row>
			</Group>
		</aside>
	)
}
