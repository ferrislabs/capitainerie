import { Link } from '@tanstack/react-router'
import { FolderKanban, Plus, Users } from 'lucide-react'
import { useState } from 'react'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import { Skeleton } from '#/components/ui/skeleton'
import { useClient } from '#/domain/clients/hooks/use-directory'
import {
	useCreateProject,
	useUpdateClient,
} from '#/domain/clients/hooks/use-directory-mutations'
import { projectBody } from '#/domain/clients/project-body'
import { CardsByColumn } from '#/domain/clients/ui/cards-by-column'
import { ClientDialog } from '#/domain/clients/ui/client-dialog'
import { EntityCard } from '#/domain/clients/ui/entity-card'
import { ProjectDialog } from '#/domain/clients/ui/project-dialog'
import { useCards } from '#/domain/kanban/hooks/use-cards'
import { useRuns } from '#/domain/runs/hooks/use-runs'
import { RunsTable } from '#/domain/runs/ui/runs-table'
import { isApiRequestError } from '#/lib/api-error'

interface Props {
	clientId: string
}

export function ClientDetailPage({ clientId }: Props) {
	const detail = useClient(clientId)
	const cards = useCards()
	const runs = useRuns()
	const update = useUpdateClient()
	const createProject = useCreateProject()
	const [editing, setEditing] = useState(false)
	const [addingProject, setAddingProject] = useState(false)

	if (isApiRequestError(detail.error) && detail.error.status === 404) {
		return (
			<Page>
				<EmptyState
					icon={Users}
					title="Client introuvable"
					description="Ce client n’existe pas dans votre organisation."
					action={
						<Button asChild variant="outline">
							<Link to="/clients">Retour aux clients</Link>
						</Button>
					}
				/>
			</Page>
		)
	}
	if (detail.isError) {
		return (
			<Page>
				<InlineAlert tone="danger">
					Impossible de charger le client.
				</InlineAlert>
			</Page>
		)
	}
	if (detail.isPending) {
		return (
			<Page>
				<output className="flex flex-col gap-3">
					<span className="sr-only">Chargement…</span>
					<Skeleton className="h-8 w-64" />
					<Skeleton className="h-32 w-full rounded-lg" />
				</output>
			</Page>
		)
	}

	const { client, projects, cards: counts } = detail.data
	const linkedCards =
		cards.data?.items.filter((card) => card.client?.id === client.id) ?? []
	const linkedRuns =
		runs.data?.items.filter((run) => run.client?.id === client.id) ?? []

	return (
		<Page>
			<PageHeader
				leading={
					<Link
						to="/clients"
						className="w-fit text-[13px] text-muted-foreground hover:text-foreground hover:underline"
					>
						Clients
					</Link>
				}
				title={client.name}
				meta={client.archived ? <Badge variant="muted">Archivé</Badge> : null}
				description={
					client.aliases.length > 0
						? `Aussi appelé : ${client.aliases.join(', ')}`
						: undefined
				}
				actions={
					<>
						<Button
							type="button"
							variant="outline"
							onClick={() => setEditing(true)}
						>
							Modifier
						</Button>
						<Button
							type="button"
							variant="outline"
							disabled={update.isPending}
							onClick={() =>
								update.mutate({
									path: { client_id: client.id },
									body: { archived: !client.archived },
								})
							}
						>
							{client.archived ? 'Restaurer' : 'Archiver'}
						</Button>
					</>
				}
			/>
			{update.errorMessage && !editing ? (
				<InlineAlert tone="danger">{update.errorMessage}</InlineAlert>
			) : null}
			{client.notes ? (
				<p className="max-w-[70ch] text-[13px] leading-relaxed whitespace-pre-wrap">
					{client.notes}
				</p>
			) : null}
			<SectionCard
				title="Projets"
				bare
				actions={
					<Button
						type="button"
						size="sm"
						variant="outline"
						disabled={client.archived}
						onClick={() => setAddingProject(true)}
					>
						<Plus aria-hidden="true" />
						Nouveau projet
					</Button>
				}
			>
				{projects.length === 0 ? (
					<EmptyState
						icon={FolderKanban}
						title="Aucun projet"
						description="Ce client n’a pas encore de projet."
					/>
				) : (
					<ul className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
						{projects.map((project) => (
							<li key={project.id} className="min-w-0">
								<EntityCard
									to="/projects/$projectId"
									params={{ projectId: project.id }}
									icon={FolderKanban}
									name={project.name}
									archived={project.archived}
									facts={[]}
									hint={project.repository ?? undefined}
								/>
							</li>
						))}
					</ul>
				)}
			</SectionCard>
			<SectionCard title="Cartes par colonne" bare>
				<CardsByColumn counts={counts} cards={linkedCards} />
			</SectionCard>
			<SectionCard
				title="Derniers runs"
				description="Parmi les 20 runs les plus récents."
				bare
			>
				<RunsTable runs={linkedRuns} />
			</SectionCard>
			<ClientDialog
				open={editing}
				client={client}
				isPending={update.isPending}
				errorMessage={update.errorMessage}
				onClose={() => {
					update.reset()
					setEditing(false)
				}}
				onSubmit={(values) =>
					update.mutate(
						{ path: { client_id: client.id }, body: values },
						{ onSuccess: () => setEditing(false) },
					)
				}
			/>
			<ProjectDialog
				open={addingProject}
				clients={[client]}
				defaultClientId={client.id}
				isPending={createProject.isPending}
				errorMessage={createProject.errorMessage}
				onClose={() => {
					createProject.reset()
					setAddingProject(false)
				}}
				onSubmit={(values) =>
					createProject.mutate(
						{ body: projectBody(values) },
						{ onSuccess: () => setAddingProject(false) },
					)
				}
			/>
		</Page>
	)
}
