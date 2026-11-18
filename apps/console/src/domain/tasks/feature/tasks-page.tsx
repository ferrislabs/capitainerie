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
import { type ReactNode, useMemo, useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import { Skeleton } from '#/components/ui/skeleton'
import { TextInput } from '#/components/ui/text-input'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { NewTaskDialog } from '#/domain/tasks/feature/new-task-dialog'
import { useAgentOf } from '#/domain/tasks/feature/use-agent-of'
import { useCards } from '#/domain/tasks/hooks/use-tasks'
import { type Task, toTask } from '#/domain/tasks/model'
import {
	activeFilterCount,
	applyFilters,
	BOARD_STATUSES,
	filterTasks,
	GROUPING_LABEL,
	groupTasks,
	NONE,
	SORT_LABEL,
	STATUS_LABEL,
	STATUS_ORDER,
	sortTasks,
	type TaskGrouping,
	type TaskSort,
} from '#/domain/tasks/task-list'
import {
	DEFAULT_GROUPING,
	DEFAULT_SORT,
	filtersOf,
	type TasksSearch,
} from '#/domain/tasks/task-search'
import { TasksBoard, TasksList } from '#/domain/tasks/ui/tasks-view'
import { cn } from '#/lib/utils'

interface Props {
	search: TasksSearch
}

type Panel = 'filter' | 'sort' | 'group'

const TOGGLE =
	'grid h-7 w-[30px] place-items-center bg-card text-muted-foreground'

function projectsOf(tasks: ReadonlyArray<Task>): Array<[string, string]> {
	const byId = new Map<string, string>()
	for (const t of tasks) if (t.projectId) byId.set(t.projectId, t.project)
	return [...byId].sort(([, a], [, b]) => a.localeCompare(b, 'fr'))
}

function Control({
	id,
	label,
	children,
}: {
	id: string
	label: string
	children: ReactNode
}) {
	return (
		<span className="flex items-center gap-1.5 text-xs text-muted-foreground">
			<label htmlFor={id}>{label}</label>
			{children}
		</span>
	)
}

export function TasksPage({ search }: Props) {
	const { vue: view, q: query } = search
	const sort = search.tri ?? DEFAULT_SORT
	const grouping = search.groupe ?? DEFAULT_GROUPING
	const filters = filtersOf(search)
	const cards = useCards()
	const team = useTeamAgents().data?.items ?? []
	const tasks = useMemo(
		() => (cards.data?.items ?? []).map((card) => toTask(card)),
		[cards.data],
	)
	const agentOf = useAgentOf()
	const navigate = useNavigate()
	const [text, setText] = useState(query)
	const [dialogOpen, setDialogOpen] = useState(false)
	const [panel, setPanel] = useState<Panel | null>(null)
	const update = (patch: Partial<TasksSearch>) =>
		void navigate({
			to: '/tasks',
			search: { ...search, q: text, ...patch },
			replace: true,
		})
	const togglePanel = (next: Panel) => setPanel(panel === next ? null : next)
	const visible = sortTasks(
		applyFilters(
			filterTasks(tasks, text, (id) => agentOf(id).name),
			filters,
		),
		sort,
	)
	const open = tasks.filter(
		(t) => t.status !== 'done' && t.status !== 'cancelled',
	).length
	const board = view === 'tableau'
	const groups = groupTasks(visible, grouping, {
		agentName: (id) => agentOf(id).name,
		...(board
			? {
					keepEmpty: true,
					statuses: filters.status ? [filters.status] : BOARD_STATUSES,
				}
			: {}),
	})
	const filterCount = activeFilterCount(filters)
	const projects = projectsOf(tasks)
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
						update({ q: next })
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
						search={{ ...search, vue: 'liste', q: text }}
						aria-label="Liste"
						aria-current={board ? undefined : 'page'}
						className={cn(TOGGLE, !board && 'bg-secondary text-foreground')}
					>
						<List aria-hidden="true" className="size-4" />
					</Link>
					<Link
						to="/tasks"
						search={{ ...search, vue: 'tableau', q: text }}
						aria-label="Tableau"
						aria-current={board ? 'page' : undefined}
						className={cn(TOGGLE, board && 'bg-secondary text-foreground')}
					>
						<Columns3 aria-hidden="true" className="size-4" />
					</Link>
				</nav>
				<Button
					variant="outline"
					aria-expanded={panel === 'filter'}
					aria-controls="tasks-options"
					onClick={() => togglePanel('filter')}
				>
					<Filter aria-hidden="true" />
					Filtrer
					{filterCount ? (
						<span className="text-muted-foreground tabular-nums">
							{filterCount}
						</span>
					) : null}
				</Button>
				<Button
					variant="outline"
					aria-expanded={panel === 'sort'}
					aria-controls="tasks-options"
					onClick={() => togglePanel('sort')}
				>
					<ArrowUpDown aria-hidden="true" />
					Trier
				</Button>
				<Button
					variant="outline"
					aria-expanded={panel === 'group'}
					aria-controls="tasks-options"
					onClick={() => togglePanel('group')}
				>
					<Layers aria-hidden="true" />
					Grouper
				</Button>
			</div>
			{panel ? (
				<div
					id="tasks-options"
					className="mb-3 flex flex-wrap items-center gap-3 rounded-lg border bg-card px-3 py-2"
				>
					{panel === 'filter' ? (
						<>
							<Control id="tasks-filter-status" label="Statut">
								<NativeSelect
									id="tasks-filter-status"
									value={filters.status ?? ''}
									onChange={(event) =>
										update({
											statut:
												STATUS_ORDER.find((s) => s === event.target.value) ??
												undefined,
										})
									}
								>
									<option value="">Tous</option>
									{STATUS_ORDER.map((s) => (
										<option key={s} value={s}>
											{STATUS_LABEL[s]}
										</option>
									))}
								</NativeSelect>
							</Control>
							<Control id="tasks-filter-agent" label="Agent">
								<NativeSelect
									id="tasks-filter-agent"
									value={filters.agent ?? ''}
									onChange={(event) =>
										update({ agent: event.target.value || undefined })
									}
								>
									<option value="">Tous</option>
									<option value={NONE}>Non assignée</option>
									{team.map((a) => (
										<option key={a.id} value={a.id}>
											{agentOf(a.id).name}
										</option>
									))}
								</NativeSelect>
							</Control>
							<Control id="tasks-filter-project" label="Projet">
								<NativeSelect
									id="tasks-filter-project"
									value={filters.project ?? ''}
									onChange={(event) =>
										update({ projet: event.target.value || undefined })
									}
								>
									<option value="">Tous</option>
									<option value={NONE}>Sans projet</option>
									{projects.map(([id, name]) => (
										<option key={id} value={id}>
											{name}
										</option>
									))}
								</NativeSelect>
							</Control>
							{filterCount ? (
								<Button
									variant="ghost"
									onClick={() =>
										update({
											statut: undefined,
											agent: undefined,
											projet: undefined,
										})
									}
								>
									Effacer les filtres
								</Button>
							) : null}
						</>
					) : panel === 'sort' ? (
						<Control id="tasks-sort" label="Trier par">
							<NativeSelect
								id="tasks-sort"
								value={sort}
								onChange={(event) =>
									update({ tri: event.target.value as TaskSort })
								}
							>
								{Object.entries(SORT_LABEL).map(([value, label]) => (
									<option key={value} value={value}>
										{label}
									</option>
								))}
							</NativeSelect>
						</Control>
					) : (
						<Control id="tasks-group" label="Grouper par">
							<NativeSelect
								id="tasks-group"
								value={grouping}
								onChange={(event) =>
									update({ groupe: event.target.value as TaskGrouping })
								}
							>
								{Object.entries(GROUPING_LABEL).map(([value, label]) => (
									<option key={value} value={value}>
										{label}
									</option>
								))}
							</NativeSelect>
						</Control>
					)}
				</div>
			) : null}
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
						: 'Modifie la recherche ou les filtres.'}{' '}
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
		</Page>
	)
}
