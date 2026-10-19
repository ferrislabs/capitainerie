import { Link } from '@tanstack/react-router'
import { FolderKanban } from 'lucide-react'
import { useState } from 'react'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import { Skeleton } from '#/components/ui/skeleton'
import { useClients, useProject } from '#/domain/clients/hooks/use-directory'
import { useUpdateProject } from '#/domain/clients/hooks/use-directory-mutations'
import { projectPatch } from '#/domain/clients/project-body'
import { CardsByColumn } from '#/domain/clients/ui/cards-by-column'
import { ProjectDialog } from '#/domain/clients/ui/project-dialog'
import { useCards } from '#/domain/kanban/hooks/use-cards'
import { useRuns } from '#/domain/runs/hooks/use-runs'
import { RunsTable } from '#/domain/runs/ui/runs-table'
import { isApiRequestError } from '#/lib/api-error'

interface Props {
	projectId: string
}

export function ProjectDetailPage({ projectId }: Props) {
	const detail = useProject(projectId)
	const clients = useClients()
	const cards = useCards()
	const runs = useRuns()
	const update = useUpdateProject()
	const [editing, setEditing] = useState(false)

	if (isApiRequestError(detail.error) && detail.error.status === 404) {
		return (
			<Page>
				<EmptyState
					icon={FolderKanban}
					title="Projet introuvable"
					description="Ce projet n’existe pas dans votre organisation."
					action={
						<Button asChild variant="outline">
							<Link to="/projects">Retour aux projets</Link>
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
					Impossible de charger le projet.
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

	const { project, cards: counts } = detail.data
	const linkedCards =
		cards.data?.items.filter((card) => card.project?.id === project.id) ?? []
	const linkedRuns =
		runs.data?.items.filter((run) => run.project?.id === project.id) ?? []

	return (
		<Page>
			<PageHeader
				leading={
					<Link
						to="/projects"
						className="w-fit text-[13px] text-muted-foreground hover:text-foreground hover:underline"
					>
						Projets
					</Link>
				}
				title={project.name}
				meta={project.archived ? <Badge variant="muted">Archivé</Badge> : null}
				description={
					project.client ? (
						<>
							Client :{' '}
							<Link
								to="/clients/$clientId"
								params={{ clientId: project.client.id }}
								className="text-link hover:underline"
							>
								{project.client.name}
							</Link>
						</>
					) : (
						'Projet interne'
					)
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
									path: { project_id: project.id },
									body: { archived: !project.archived },
								})
							}
						>
							{project.archived ? 'Restaurer' : 'Archiver'}
						</Button>
					</>
				}
			/>
			{update.errorMessage && !editing ? (
				<InlineAlert tone="danger">{update.errorMessage}</InlineAlert>
			) : null}
			{project.repository ? (
				<p className="font-mono text-xs text-muted-foreground">
					{project.repository}
				</p>
			) : null}
			{project.description ? (
				<p className="max-w-[70ch] text-[13px] leading-relaxed whitespace-pre-wrap">
					{project.description}
				</p>
			) : null}
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
			<ProjectDialog
				open={editing}
				project={project}
				clients={clients.data?.items.map((item) => item.client) ?? []}
				isPending={update.isPending}
				errorMessage={update.errorMessage}
				onClose={() => {
					update.reset()
					setEditing(false)
				}}
				onSubmit={(values) =>
					update.mutate(
						{ path: { project_id: project.id }, body: projectPatch(values) },
						{ onSuccess: () => setEditing(false) },
					)
				}
			/>
		</Page>
	)
}
