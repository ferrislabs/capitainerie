import { Link, useNavigate } from '@tanstack/react-router'
import {
	ArrowUpDown,
	Columns3,
	Filter,
	Layers,
	List,
	Plus,
	Search,
} from 'lucide-react'
import { useState } from 'react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { useNotice } from '#/components/layout/notice'
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
import { Toast } from '#/domain/tasks/ui/toast'
import { cn } from '#/lib/utils'

interface Props {
	view: TasksView
	query: string
}

const TOGGLE =
	'grid h-7 w-[30px] place-items-center bg-card text-muted-foreground'

export function TasksPage({ view, query }: Props) {
	const tasks = useDemoState((s) => s.tasks)
	const agentOf = useAgentOf()
	const navigate = useNavigate()
	const [text, setText] = useState(query)
	const [dialogOpen, setDialogOpen] = useState(false)
	const [message, notify] = useNotice()
	const visible = filterTasks(tasks, text, (id) => agentOf(id).name)
	const board = view === 'tableau'
	const groups = groupByStatus(
		visible,
		board ? { keepEmpty: true, statuses: BOARD_STATUSES } : {},
	)
	return (
		<Page className="leading-[1.4]">
			<DemoBanner />
			<PageHeader title="Tâches" description="122 ouvertes · 65 bloquées" />
			<div className="mb-3 flex flex-wrap items-center gap-2">
				<Button variant="outline" onClick={() => setDialogOpen(true)}>
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
					wrapperClassName="min-w-40 max-w-90 flex-1"
				/>
				<span className="flex-1" />
				<nav
					aria-label="Affichage"
					className="inline-flex overflow-hidden rounded-[7px] border"
				>
					<Link
						to="/tasks"
						search={{ vue: 'liste', q: text }}
						aria-label="Liste"
						aria-current={board ? undefined : 'page'}
						className={cn(TOGGLE, !board && 'bg-secondary text-foreground')}
					>
						<List aria-hidden="true" className="size-4" />
					</Link>
					<Link
						to="/tasks"
						search={{ vue: 'tableau', q: text }}
						aria-label="Tableau"
						aria-current={board ? 'page' : undefined}
						className={cn(TOGGLE, board && 'bg-secondary text-foreground')}
					>
						<Columns3 aria-hidden="true" className="size-4" />
					</Link>
				</nav>
				<Button
					variant="outline"
					onClick={() => notify('Filtres : statut, agent, projet, étiquette')}
				>
					<Filter aria-hidden="true" />
					Filtrer
				</Button>
				<Button
					variant="outline"
					onClick={() => notify('Tri : mise à jour, création, priorité')}
				>
					<ArrowUpDown aria-hidden="true" />
					Trier
				</Button>
				<Button
					variant="outline"
					onClick={() => notify('Regroupement : statut, agent, projet')}
				>
					<Layers aria-hidden="true" />
					Grouper
				</Button>
			</div>
			{visible.length === 0 ? (
				<div className="px-4 py-[26px] text-center text-muted-foreground">
					<b className="mt-1.5 mb-0.5 block font-semibold text-foreground">
						Aucune tâche
					</b>
					Modifie la recherche.
				</div>
			) : board ? (
				<TasksBoard
					groups={groups}
					agentOf={agentOf}
					onAdd={() => setDialogOpen(true)}
				/>
			) : (
				<TasksList groups={groups} agentOf={agentOf} />
			)}
			<NewTaskDialog open={dialogOpen} onOpenChange={setDialogOpen} />
			<Toast message={message} />
		</Page>
	)
}
