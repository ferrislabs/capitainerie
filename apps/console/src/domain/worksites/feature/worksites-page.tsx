import { Link } from '@tanstack/react-router'
import { FolderKanban, HardHat, Plus } from 'lucide-react'
import { useState } from 'react'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Notice, useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import { Button } from '#/components/ui/button'
import { Combobox } from '#/components/ui/combobox'
import { Field } from '#/components/ui/field'
import { Skeleton } from '#/components/ui/skeleton'
import { RepositoriesSection } from '#/domain/integrations/feature/repositories-section'
import { useProjects } from '#/domain/projects/hooks/use-projects'
import { Empty } from '#/domain/sources/ui/kit'
import {
	resolveCurrentProject,
	useCurrentProject,
} from '#/domain/worksites/current-project'
import { LaunchWorksitesFlow } from '#/domain/worksites/feature/launch-worksites-flow'
import { useWorksites } from '#/domain/worksites/hooks/use-worksites'
import { WorksitesTable } from '#/domain/worksites/ui/worksites-table'
import { useNow } from '#/hooks/use-now'

const MINUTE_MS = 60_000

function launchedMessage(count: number): string {
	return count > 1
		? `${count} chantiers lancés. Ils attendent leur tour si les workers sont occupés.`
		: '1 chantier lancé. Il attend son tour si les workers sont occupés.'
}

export function WorksitesPage() {
	const projects = useProjects(false)
	const [stored, setCurrent] = useCurrentProject()
	const [launching, setLaunching] = useState(false)
	const [notice, notify] = useNotice(6000)
	const now = useNow(true, MINUTE_MS)

	const all = projects.data?.items.map((item) => item.project) ?? []
	const currentId = resolveCurrentProject(stored, all)
	const current = all.find((project) => project.id === currentId)
	const worksites = useWorksites(currentId)
	const items = worksites.data?.items ?? []

	return (
		<Page>
			<PageHeader
				title="Chantiers"
				description={
					current && worksites.data
						? `Travail confié à Claude Code · ${items.length} chantier${items.length > 1 ? 's' : ''} sur ${current.name}`
						: 'Travail confié à Claude Code, projet par projet'
				}
				actions={
					<Button
						type="button"
						disabled={current === undefined}
						onClick={() => setLaunching(true)}
					>
						<Plus aria-hidden="true" />
						Nouveau chantier
					</Button>
				}
			/>
			<Notice message={notice} className="mb-3" />
			{projects.isError ? (
				<div className="grid gap-2">
					<InlineAlert tone="danger">
						Impossible de charger les projets.
					</InlineAlert>
					<div>
						<Button variant="outline" onClick={() => void projects.refetch()}>
							Réessayer
						</Button>
					</div>
				</div>
			) : projects.isPending ? (
				<output className="flex flex-col gap-2">
					<span className="sr-only">Chargement…</span>
					<Skeleton className="h-8 w-72" />
				</output>
			) : all.length === 0 ? (
				<EmptyState
					icon={FolderKanban}
					title="Aucun projet"
					description="Un chantier se lance dans un projet. Crée-en un d'abord."
					action={
						<Button asChild variant="outline">
							<Link to="/projects">Voir les projets</Link>
						</Button>
					}
				/>
			) : (
				<>
					<Field
						label="Projet"
						id="worksites-project"
						className="mb-3 max-w-sm"
					>
						<Combobox
							value={currentId}
							onValueChange={setCurrent}
							items={all.map((project) => ({
								value: project.id,
								label: project.client
									? `${project.name} · ${project.client.name}`
									: project.name,
							}))}
							placeholder="Choisir un projet…"
							emptyText="Aucun projet ne correspond"
						/>
					</Field>
					{current === undefined ? (
						<EmptyState
							icon={HardHat}
							title="Choisis un projet"
							description="Les chantiers s'affichent par projet. Ton choix est retenu pour la prochaine visite."
						/>
					) : worksites.isError ? (
						<div className="grid gap-2">
							<InlineAlert tone="danger">
								Impossible de charger les chantiers de ce projet.
							</InlineAlert>
							<div>
								<Button
									variant="outline"
									onClick={() => void worksites.refetch()}
								>
									Réessayer
								</Button>
							</div>
						</div>
					) : worksites.isPending ? (
						<TableSkeleton rows={3} columns={5} />
					) : (
						<div className="mb-3 overflow-hidden rounded-lg border bg-card">
							{items.length === 0 ? (
								<Empty title="Aucun chantier">
									<span className="inline-flex items-center gap-1.5">
										<HardHat aria-hidden="true" className="size-4" />
										Lance un ou plusieurs chantiers depuis des prompts ou des
										issues GitHub.
									</span>
								</Empty>
							) : (
								<WorksitesTable items={items} now={now} />
							)}
						</div>
					)}
					<RepositoriesSection />
				</>
			)}
			{current ? (
				<LaunchWorksitesFlow
					open={launching}
					project={current}
					defaultRepository={current.repository ?? null}
					onClose={() => setLaunching(false)}
					onLaunched={(count) => {
						setLaunching(false)
						notify(launchedMessage(count))
					}}
				/>
			) : null}
		</Page>
	)
}
