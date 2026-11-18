import type { Schemas } from '#/api/api.client'
import { Button } from '#/components/ui/button'
import { DetailSection } from '#/domain/agent-team/ui/panel'
import { DETAIL_ROW, Empty, Rows, Tag, Ttl } from '#/domain/agent-team/ui/rows'
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
		<DetailSection title="Révisions" flush>
			{error ? (
				<p role="alert" className="m-0 px-6 py-5 text-signal-danger">
					Impossible de charger les révisions.
				</p>
			) : loading ? (
				<output className="m-0 block px-6 py-5 text-muted-foreground">
					Chargement des révisions…
				</output>
			) : revisions.length === 0 ? (
				<Empty title="Aucune révision">
					Chaque enregistrement de cet agent crée une révision.
				</Empty>
			) : (
				<Rows>
					{revisions.map((revision) => (
						<div key={revision.revision} className={DETAIL_ROW}>
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
		</DetailSection>
	)
}
