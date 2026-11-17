import { Link, useNavigate } from '@tanstack/react-router'
import { Columns3, List, Plus, Search } from 'lucide-react'
import { useState } from 'react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { EmptyState } from '#/components/layout/empty-state'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { TextInput } from '#/components/ui/text-input'
import { useDemoState } from '#/domain/demo'
import { NewTaskDialog } from '#/domain/tasks/feature/new-task-dialog'
import { useAgentOf } from '#/domain/tasks/feature/use-agent-of'
import {
	BOARD_STATUSES,
	filterTasks,
	groupByStatus,
} from '#/domain/tasks/task-list'
import type { TasksView } from '#/domain/tasks/task-search'
import { TasksBoard, TasksList } from '#/domain/tasks/ui/tasks-view'
import { cn } from '#/lib/utils'

interface Props {
	view: TasksView
	query: string
}

const TOGGLE =
	'inline-flex size-7 items-center justify-center rounded-md border'

export function TasksPage({ view, query }: Props) {
	const tasks = useDemoState((s) => s.tasks)
	const agentOf = useAgentOf()
	const navigate = useNavigate()
	const [text, setText] = useState(query)
	const [dialogOpen, setDialogOpen] = useState(false)
	const visible = filterTasks(tasks, text, (id) => agentOf(id).name)
	const open = tasks.filter(
		(t) => t.status !== 'done' && t.status !== 'cancelled',
	).length
	const blocked = tasks.filter((t) => t.status === 'blocked').length
	const board = view === 'tableau'
	const groups = groupByStatus(
		visible,
		board ? { keepEmpty: true, statuses: BOARD_STATUSES } : {},
	)
	return (
		<Page>
			<DemoBanner />
			<PageHeader
				title="Tâches"
				description={`${open} ouvertes · ${blocked} bloquées`}
			/>
			<div className="flex flex-wrap items-center gap-2">
				<Button onClick={() => setDialogOpen(true)}>
					<Plus aria-hidden="true" />
					Nouvelle tâche
				</Button>
				<TextInput
					aria-label="Rechercher des tâches"
					placeholder="Rechercher des tâches…"
					leading={<Search aria-hidden="true" />}
					value={text}
					onChange={(event) => {
						const next = event.target.value
						setText(next)
						void navigate({
							to: '/tasks',
							search: { vue: view, q: next },
							replace: true,
						})
					}}
					wrapperClassName="w-64"
				/>
				<span className="flex-1" />
				<nav aria-label="Affichage" className="flex gap-1">
					<Link
						to="/tasks"
						search={{ vue: 'liste', q: text }}
						aria-label="Liste"
						aria-current={board ? undefined : 'page'}
						className={cn(TOGGLE, !board && 'bg-muted')}
					>
						<List aria-hidden="true" className="size-4" />
					</Link>
					<Link
						to="/tasks"
						search={{ vue: 'tableau', q: text }}
						aria-label="Tableau"
						aria-current={board ? 'page' : undefined}
						className={cn(TOGGLE, board && 'bg-muted')}
					>
						<Columns3 aria-hidden="true" className="size-4" />
					</Link>
				</nav>
			</div>
			{visible.length === 0 ? (
				<EmptyState
					icon={Search}
					title="Aucune tâche"
					description="Modifie la recherche."
				/>
			) : board ? (
				<TasksBoard groups={groups} agentOf={agentOf} />
			) : (
				<TasksList groups={groups} agentOf={agentOf} />
			)}
			<NewTaskDialog open={dialogOpen} onOpenChange={setDialogOpen} />
		</Page>
	)
}
