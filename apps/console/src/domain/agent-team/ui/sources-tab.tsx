import { Link } from '@tanstack/react-router'
import { Cable } from 'lucide-react'
import { DetailSection } from '#/domain/agent-team/ui/panel'
import { DETAIL_ROW, Empty, Ttl } from '#/domain/agent-team/ui/rows'
import { relativeAge } from '#/domain/dashboard/format'
import { useSources } from '#/domain/sources/hooks/use-sources'
import { sourcesOfAgent } from '#/domain/sources/model'

interface Props {
	agentId: string
}

export function SourcesTab({ agentId }: Props) {
	const query = useSources()
	const sources = sourcesOfAgent(query.data?.items ?? [], agentId)
	return (
		<DetailSection
			title="Sources écoutées"
			flush
			actions={<Link to="/sources/nouvelle">Nouvelle source</Link>}
		>
			{query.isPending ? (
				<output className="block px-6 py-5 text-muted-foreground">
					Chargement des sources…
				</output>
			) : query.isError ? (
				<Empty title="Impossible de charger les sources" />
			) : sources.length === 0 ? (
				<Empty
					icon={<Cable aria-hidden="true" className="mx-auto size-4" />}
					title="Aucune source branchée"
				>
					Aucune source n'a cet agent comme agent par défaut. Choisis-le dans
					les paramètres d'une source pour qu'il reçoive ses événements.
				</Empty>
			) : (
				<ul className="m-0 list-none p-0">
					{sources.map((source) => (
						<li key={source.id} className={DETAIL_ROW}>
							<Ttl
								title={
									<Link
										to="/sources/$sourceId/{-$tab}"
										params={{ sourceId: source.id }}
										className="text-foreground"
									>
										{source.name}
									</Link>
								}
								sub={
									source.last_event_at
										? `Dernier événement ${relativeAge(source.last_event_at)}`
										: 'Aucun événement reçu'
								}
							/>
							<span className="text-[12px] whitespace-nowrap text-muted-foreground">
								{source.enabled
									? `${source.events_last_24h} / 24 h`
									: 'En pause'}
							</span>
						</li>
					))}
				</ul>
			)}
		</DetailSection>
	)
}
