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
import { useMemo, useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { Skeleton } from '#/components/ui/skeleton'
import { TextInput } from '#/components/ui/text-input'
import { NewTaskDialog } from '#/domain/tasks/feature/new-task-dialog'
import { useAgentOf } from '#/domain/tasks/feature/use-agent-of'
import { useCards } from '#/domain/tasks/hooks/use-tasks'
import { toTask } from '#/domain/tasks/model'
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
	const cards = useCards()
	const tasks = useMemo(
		() => (cards.data?.items ?? []).map((card) => toTask(card)),
		[cards.data],
	)
	const agentOf = useAgentOf()
	const navigate = useNavigate()
	const [text, setText] = useState(query)
	const [dialogOpen, setDialogOpen] = useState(false)
	const [message, notify] = useNotice()
	const visible = filterTasks(tasks, text, (id) => agentOf(id).name)
	const open = tasks.filter(
		(t) => t.status !== 'done' && t.status !== 'cancelled',
	).length
	const board = view === 'tableau'
	const groups = groupByStatus(
		visible,
		board ? { keepEmpty: true, statuses: BOARD_STATUSES } : {},
	)
	return (
		<Page className="leading-[1.4]">
			<PageHeader
				title="Tâches"
				description={cards.data ? `${open} ouvertes` : undefined}
			/>
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
			{cards.isError ? (
				<InlineAlert tone="danger">
					Impossible de charger les tâches.
				</InlineAlert>
			) : cards.isPending ? (
				<output className="grid gap-2">
					<span className="sr-only">Chargement…</span>
					{['a', 'b', 'c'].map((key) => (
						<Skeleton key={key} className="h-9 rounded-md" />
					))}
				</output>
			) : visible.length === 0 ? (
				<div className="px-4 py-[26px] text-center text-muted-foreground">
					{' '}
					<b className="mt-1.5 mb-0.5 block font-semibold text-foreground">
						{' '}
						Aucune tâche{' '}
					</b>{' '}
					{tasks.length === 0
						? 'Crée une tâche pour la confier à un agent.'
						: 'Modifie la recherche.'}{' '}
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
