import { Link } from '@tanstack/react-router'
import { X } from 'lucide-react'
import type { ReactNode } from 'react'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import type { DemoTask, DemoTaskStatus } from '#/domain/demo'
import { taskPathId } from '#/domain/tasks/task-ids'
import { STATUS_LABEL, STATUS_ORDER } from '#/domain/tasks/task-list'
import { AgentChip } from '#/domain/tasks/ui/agent-chip'
import type { AgentView } from '#/domain/tasks/ui/tasks-view'

interface Props {
	task: DemoTask
	agent: AgentView
	subtasks: ReadonlyArray<DemoTask>
	onStatus: (status: DemoTaskStatus) => void
	onClose: () => void
}

function Row({ label, children }: { label: string; children: ReactNode }) {
	return (
		<div className="flex items-start justify-between gap-3 py-1 text-[13px]">
			<span className="text-muted-foreground">{label}</span>
			<span className="min-w-0 text-right break-words">{children}</span>
		</div>
	)
}

const LINK = 'text-signal-running hover:underline'

export function PropertiesPanel({
	task,
	agent,
	subtasks,
	onStatus,
	onClose,
}: Props) {
	return (
		<aside
			aria-label="Propriétés"
			className="flex flex-col gap-1 rounded-lg border bg-card p-3"
		>
			<header className="flex items-center justify-between">
				<h2 className="text-[13px] font-semibold">Propriétés</h2>
				<Button
					variant="ghost"
					size="icon-sm"
					aria-label="Masquer les propriétés"
					onClick={onClose}
				>
					<X aria-hidden="true" />
				</Button>
			</header>
			<Row label="Statut">
				<NativeSelect
					aria-label="Statut"
					value={task.status}
					onChange={(event) => onStatus(event.target.value as DemoTaskStatus)}
				>
					{STATUS_ORDER.map((s) => (
						<option key={s} value={s}>
							{STATUS_LABEL[s]}
						</option>
					))}
				</NativeSelect>
			</Row>
			<Row label="Assigné à">
				<Link
					to="/agents/equipe/$agentId/{-$tab}"
					params={{ agentId: task.agentId }}
					className={LINK}
				>
					<AgentChip name={agent.name} hue={agent.hue} />
				</Link>
			</Row>
			<Row label="Projet">{task.project || 'Aucun'}</Row>
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
			<Row label="Sous-tâches">
				{subtasks.length ? (
					<span className="flex flex-wrap justify-end gap-x-2">
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
			<Row label="Origine">
				{task.sourceId ? 'Source' : 'François-Guillaume Ribreau'}
			</Row>
			<Row label="Âge">{task.age}</Row>
		</aside>
	)
}
