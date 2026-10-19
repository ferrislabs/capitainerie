import { Plus, Users } from 'lucide-react'
import { useState } from 'react'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { Skeleton } from '#/components/ui/skeleton'
import { useClients } from '#/domain/clients/hooks/use-directory'
import { useCreateClient } from '#/domain/clients/hooks/use-directory-mutations'
import { ClientDialog } from '#/domain/clients/ui/client-dialog'
import { DirectoryToolbar } from '#/domain/clients/ui/directory-toolbar'
import { EntityCard } from '#/domain/clients/ui/entity-card'

export function ClientsPage() {
	const [query, setQuery] = useState('')
	const [showArchived, setShowArchived] = useState(false)
	const [creating, setCreating] = useState(false)
	const clients = useClients({ query, archived: showArchived })
	const create = useCreateClient()
	const filtering = query.trim() !== '' || showArchived

	function closeDialog() {
		create.reset()
		setCreating(false)
	}

	return (
		<Page>
			<PageHeader
				title="Clients"
				description="Les organisations et les personnes pour qui l’équipe travaille. Une carte peut être rattachée à un client, au dépôt ou plus tard."
				actions={
					<Button type="button" onClick={() => setCreating(true)}>
						<Plus aria-hidden="true" />
						Nouveau client
					</Button>
				}
			/>
			<DirectoryToolbar
				label="Rechercher un client"
				query={query}
				showArchived={showArchived}
				onQueryChange={setQuery}
				onShowArchivedChange={setShowArchived}
			/>
			{clients.isError ? (
				<InlineAlert tone="danger">
					Impossible de charger les clients.
				</InlineAlert>
			) : clients.isPending ? (
				<output className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
					<span className="sr-only">Chargement…</span>
					<Skeleton className="h-24 rounded-lg" />
					<Skeleton className="h-24 rounded-lg" />
				</output>
			) : clients.data.items.length === 0 ? (
				<EmptyState
					icon={Users}
					title={filtering ? 'Aucun client trouvé' : 'Aucun client'}
					description={
						filtering
							? 'Aucun client ne correspond à cette recherche.'
							: 'Créez un premier client pour y rattacher des projets et des cartes.'
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
								Nouveau client
							</Button>
						)
					}
				/>
			) : (
				<ul className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
					{clients.data.items.map(({ client, project_count, open_cards }) => (
						<li key={client.id} className="min-w-0">
							<EntityCard
								to="/clients/$clientId"
								params={{ clientId: client.id }}
								icon={Users}
								name={client.name}
								archived={client.archived}
								facts={[
									{
										label: project_count > 1 ? 'projets' : 'projet',
										value: project_count,
									},
									{
										label: open_cards > 1 ? 'cartes ouvertes' : 'carte ouverte',
										value: open_cards,
									},
								]}
								hint={client.aliases.join(', ') || undefined}
							/>
						</li>
					))}
				</ul>
			)}
			<ClientDialog
				open={creating}
				isPending={create.isPending}
				errorMessage={create.errorMessage}
				onClose={closeDialog}
				onSubmit={(values) =>
					create.mutate(
						{ body: values },
						{ onSuccess: () => setCreating(false) },
					)
				}
			/>
		</Page>
	)
}
