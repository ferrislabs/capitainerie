import type { Schemas } from '#/api/api.client'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { Empty, ROW, Rows, Tag, Ttl } from '#/domain/agent-team/ui/rows'
import { Sev } from '#/domain/agent-team/ui/sev'
import { relativeAge } from '#/domain/dashboard/format'

interface Props {
	current: number
	revisions: ReadonlyArray<Schemas.TeamAgentRevisionResponse>
	loading: boolean
	error: boolean
	restoring: boolean
	onRestore: (revision: number) => void
}

export function RevisionsTab({
	current,
	revisions,
	loading,
	error,
	restoring,
	onRestore,
}: Props) {
	return (
		<SectionCard title="Révisions" flush>
			{error ? (
				<p role="alert" className="m-0 p-3 text-signal-danger">
					Impossible de charger les révisions.
				</p>
			) : loading ? (
				<output className="m-0 block p-3 text-muted-foreground">
					Chargement des révisions…
				</output>
			) : revisions.length === 0 ? (
				<Empty title="Aucune révision">
					Chaque enregistrement de cet agent crée une révision.
				</Empty>
			) : (
				<Rows>
					{revisions.map((revision) => (
						<div key={revision.revision} className={ROW}>
							<span className="w-[54px]">
								<Tag>Rév. {revision.revision}</Tag>
							</span>
							<Ttl
								title={`Modifié : ${revision.changed || '—'}`}
								sub={`${revision.author} · ${relativeAge(revision.created_at)}`}
							/>
							{revision.revision === current ? (
								<Sev tone="ok">Actuelle</Sev>
							) : (
								<Button
									type="button"
									variant="outline"
									size="sm"
									disabled={restoring}
									aria-label={`Restaurer la révision ${revision.revision}`}
									onClick={() => onRestore(revision.revision)}
								>
									Restaurer
								</Button>
							)}
						</div>
					))}
				</Rows>
			)}
		</SectionCard>
	)
}
