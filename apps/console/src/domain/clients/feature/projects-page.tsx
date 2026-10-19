import { FolderKanban, Plus } from 'lucide-react'
import { useState } from 'react'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { Skeleton } from '#/components/ui/skeleton'
import { useClients, useProjects } from '#/domain/clients/hooks/use-directory'
import { useCreateProject } from '#/domain/clients/hooks/use-directory-mutations'
import { projectBody } from '#/domain/clients/project-body'
import { DirectoryToolbar } from '#/domain/clients/ui/directory-toolbar'
import { EntityCard } from '#/domain/clients/ui/entity-card'
import { ProjectDialog } from '#/domain/clients/ui/project-dialog'

export function ProjectsPage() {
	const [query, setQuery] = useState('')
	const [showArchived, setShowArchived] = useState(false)
	const [creating, setCreating] = useState(false)
	const projects = useProjects({ query, archived: showArchived })
	const clients = useClients()
	const create = useCreateProject()
	const filtering = query.trim() !== '' || showArchived

	function closeDialog() {
		create.reset()
		setCreating(false)
	}

	return (
		<Page>
			<PageHeader
				title="Projets"
				description="Le travail suivi pour un client, ou en interne, avec un dépôt GitHub par défaut facultatif."
				actions={
					<Button type="button" onClick={() => setCreating(true)}>
						<Plus aria-hidden="true" />
						Nouveau projet
					</Button>
				}
			/>
			<DirectoryToolbar
				label="Rechercher un projet"
				query={query}
				showArchived={showArchived}
				onQueryChange={setQuery}
				onShowArchivedChange={setShowArchived}
			/>
			{projects.isError ? (
				<InlineAlert tone="danger">
					Impossible de charger les projets.
				</InlineAlert>
			) : projects.isPending ? (
				<output className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
					<span className="sr-only">Chargement…</span>
					<Skeleton className="h-24 rounded-lg" />
					<Skeleton className="h-24 rounded-lg" />
				</output>
			) : projects.data.items.length === 0 ? (
				<EmptyState
					icon={FolderKanban}
					title={filtering ? 'Aucun projet trouvé' : 'Aucun projet'}
					description={
						filtering
							? 'Aucun projet ne correspond à cette recherche.'
							: 'Créez un premier projet pour y rattacher des cartes et un dépôt par défaut.'
					}
					action={
						filtering ? (
							<Button
								type="button"
								variant="outline"
								onClick={() => {
									setQuery('')
									setShowArchived(false)
								}}
							>
								Réinitialiser les filtres
							</Button>
						) : (
							<Button type="button" onClick={() => setCreating(true)}>
								Nouveau projet
							</Button>
						)
					}
				/>
			) : (
				<ul className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
					{projects.data.items.map(({ project, open_cards }) => (
						<li key={project.id} className="min-w-0">
							<EntityCard
								to="/projects/$projectId"
								params={{ projectId: project.id }}
								icon={FolderKanban}
								name={project.name}
								archived={project.archived}
								facts={[
									{
										label: open_cards > 1 ? 'cartes ouvertes' : 'carte ouverte',
										value: open_cards,
									},
								]}
								hint={project.client?.name ?? 'Projet interne'}
							/>
						</li>
					))}
				</ul>
			)}
			<ProjectDialog
				open={creating}
				clients={clients.data?.items.map((item) => item.client) ?? []}
				isPending={create.isPending}
				errorMessage={create.errorMessage}
				onClose={closeDialog}
				onSubmit={(values) =>
					create.mutate(
						{ body: projectBody(values) },
						{ onSuccess: () => setCreating(false) },
					)
				}
			/>
		</Page>
	)
}
