import { FolderKanban, Plus } from 'lucide-react'
import { useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { Skeleton } from '#/components/ui/skeleton'
import {
	useClients,
	useCreateProject,
	useProjectCards,
	useProjects,
	useUpdateProject,
} from '#/domain/projects/hooks/use-projects'
import {
	createBody,
	draftOf,
	type ProjectDraft,
	progressByProject,
	updateBody,
} from '#/domain/projects/model'
import { ProjectCard } from '#/domain/projects/ui/project-card'
import { ProjectDialog } from '#/domain/projects/ui/project-dialog'

type Editing =
	| { mode: 'create' }
	| { mode: 'edit'; project: Schemas.ProjectResponse }

function errorMessage(error: unknown): string {
	const status = (error as { status?: number } | null)?.status
	if (status === 409) return 'Un projet porte déjà ce nom.'
	if (status === 403) return "Tu n'as pas le droit de modifier les projets."
	if (status === 422) return 'Les informations du projet sont invalides.'
	return "L'enregistrement a échoué. Réessaie."
}

export function ProjectsPage() {
	const [showArchived, setShowArchived] = useState(false)
	const [editing, setEditing] = useState<Editing | null>(null)
	const projects = useProjects(showArchived)
	const clients = useClients()
	const cards = useProjectCards()
	const create = useCreateProject()
	const update = useUpdateProject()
	const progress = cards.data ? progressByProject(cards.data.items) : null
	const items = projects.data?.items ?? []
	const mutation = editing?.mode === 'edit' ? update : create

	const close = () => {
		setEditing(null)
		create.reset()
		update.reset()
	}

	const submit = (draft: ProjectDraft) => {
		if (!editing) return
		if (editing.mode === 'create') {
			create.mutate({ body: createBody(draft) }, { onSuccess: close })
			return
		}
		update.mutate(
			{
				path: { project_id: editing.project.id },
				body: updateBody(editing.project, draft),
			},
			{ onSuccess: close },
		)
	}

	const toggleArchive = (project: Schemas.ProjectResponse) =>
		update.mutate({
			path: { project_id: project.id },
			body: { archived: !project.archived },
		})

	return (
		<Page className="leading-[1.4]">
			<PageHeader
				title="Projets"
				description={
					projects.data
						? `${items.length} ${showArchived ? (items.length > 1 ? 'projets archivés' : 'projet archivé') : items.length > 1 ? 'projets actifs' : 'projet actif'}`
						: undefined
				}
				actions={
					<>
						<Button
							variant="outline"
							aria-pressed={showArchived}
							onClick={() => setShowArchived((value) => !value)}
						>
							{showArchived ? 'Voir les actifs' : 'Voir les archivés'}
						</Button>
						<Button onClick={() => setEditing({ mode: 'create' })}>
							<Plus aria-hidden="true" />
							Nouveau projet
						</Button>
					</>
				}
			/>
			{update.isError && editing === null ? (
				<InlineAlert tone="danger" className="mb-2.5">
					{errorMessage(update.error)}
				</InlineAlert>
			) : null}
			{projects.isError ? (
				<InlineAlert tone="danger">
					Impossible de charger les projets.
				</InlineAlert>
			) : projects.isPending ? (
				<output className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-2.5">
					<span className="sr-only">Chargement…</span>
					{['a', 'b', 'c'].map((key) => (
						<Skeleton key={key} className="h-28 rounded-lg" />
					))}
				</output>
			) : items.length === 0 ? (
				<EmptyState
					icon={FolderKanban}
					title={showArchived ? 'Aucun projet archivé' : 'Aucun projet'}
					description={
						showArchived
							? 'Les projets archivés apparaissent ici.'
							: 'Crée un projet pour y rattacher des tâches et un dépôt.'
					}
				/>
			) : (
				<div className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-2.5">
					{items.map((item) => (
						<ProjectCard
							key={item.project.id}
							item={item}
							progress={progress?.get(item.project.id) ?? null}
							busy={update.isPending}
							onEdit={() => setEditing({ mode: 'edit', project: item.project })}
							onToggleArchive={() => toggleArchive(item.project)}
						/>
					))}
				</div>
			)}
			<ProjectDialog
				open={editing !== null}
				mode={editing?.mode ?? 'create'}
				initial={editing?.mode === 'edit' ? draftOf(editing.project) : null}
				clients={(clients.data?.items ?? []).map((entry) => entry.client)}
				pending={mutation.isPending}
				error={mutation.isError ? errorMessage(mutation.error) : null}
				onSubmit={submit}
				onClose={close}
			/>
		</Page>
	)
}
