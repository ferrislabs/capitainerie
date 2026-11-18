import { Link } from '@tanstack/react-router'
import { Plus, Radio, Search } from 'lucide-react'
import { useMemo, useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Notice, useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { useSources, useUpdateSource } from '#/domain/sources/hooks/use-sources'
import {
	describeSourceError,
	filterSources,
	type Source,
} from '#/domain/sources/model'
import { Empty } from '#/domain/sources/ui/kit'
import { SourcesTable } from '#/domain/sources/ui/sources-table'

function describe(sources: ReadonlyArray<Source>, archived: boolean): string {
	const total = sources.length
	const noun = total > 1 ? 'sources' : 'source'
	if (archived)
		return `${total} ${noun} ${total > 1 ? 'archivées' : 'archivée'}`
	const active = sources.filter((s) => s.enabled).length
	return `Ce que tes agents écoutent · ${total} ${noun} · ${active} ${active > 1 ? 'actives' : 'active'}`
}

export function SourcesPage() {
	const [showArchived, setShowArchived] = useState(false)
	const [query, setQuery] = useState('')
	const [notice, notify] = useNotice()
	const sources = useSources(showArchived)
	const agents = useTeamAgents().data?.items ?? []
	const update = useUpdateSource()
	const items = sources.data?.items ?? []
	const visible = useMemo(() => filterSources(items, query), [items, query])

	const toggle = (source: Source, enabled: boolean) =>
		update.mutate(
			{ path: { source_id: source.id }, body: { enabled } },
			{
				onSuccess: () =>
					notify(
						`${enabled ? 'Source activée' : 'Source en pause'} : ${source.name}`,
					),
				onError: (error) => notify(describeSourceError(error)),
			},
		)

	return (
		<Page>
			<PageHeader
				title="Sources"
				description={
					sources.isPending || sources.isError
						? undefined
						: describe(items, showArchived)
				}
				actions={
					<>
						<Button
							variant="outline"
							aria-pressed={showArchived}
							onClick={() => setShowArchived((value) => !value)}
						>
							{showArchived ? 'Voir les actives' : 'Voir les archivées'}
						</Button>
						<Button asChild>
							<Link to="/sources/nouvelle">
								<Plus aria-hidden="true" />
								Nouvelle source
							</Link>
						</Button>
					</>
				}
			/>
			<Notice message={notice} className="mb-3" />
			<label className="mb-3 flex max-w-[360px] items-center gap-2 rounded-[6px] border bg-background px-[9px] py-1.5 focus-within:border-primary">
				<Search aria-hidden="true" className="size-4 text-muted-foreground" />
				<input
					type="search"
					aria-label="Rechercher une source"
					placeholder="Rechercher…"
					value={query}
					onChange={(event) => setQuery(event.target.value)}
					className="min-w-0 flex-1 bg-transparent text-[13px] outline-none"
				/>
			</label>
			{sources.isError ? (
				<div className="grid gap-2">
					<InlineAlert tone="danger">
						Impossible de charger les sources.
					</InlineAlert>
					<div>
						<Button variant="outline" onClick={() => void sources.refetch()}>
							Réessayer
						</Button>
					</div>
				</div>
			) : sources.isPending ? (
				<output className="m-0 text-muted-foreground">
					Chargement des sources…
				</output>
			) : (
				<div className="overflow-hidden rounded-lg border bg-card">
					{visible.length === 0 ? (
						<Empty
							title={
								query.trim()
									? 'Aucune source ne correspond'
									: showArchived
										? 'Aucune source archivée'
										: 'Aucune source'
							}
						>
							{query.trim() || showArchived ? null : (
								<span className="inline-flex items-center gap-1.5">
									<Radio aria-hidden="true" className="size-4" />
									Crée un webhook pour transformer les événements de tes outils
									en tâches.
								</span>
							)}
						</Empty>
					) : (
						<SourcesTable
							sources={visible}
							agents={agents}
							busy={update.isPending}
							onToggle={toggle}
						/>
					)}
				</div>
			)}
		</Page>
	)
}
