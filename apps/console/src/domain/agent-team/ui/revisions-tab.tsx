import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import type { Revision } from '#/domain/agent-team/agent-config'
import { Empty, ROW, Rows, Tag, Ttl } from '#/domain/agent-team/ui/rows'
import { Sev } from '#/domain/agent-team/ui/sev'

interface Props {
	revisions: ReadonlyArray<Revision>
	onRestore: (label: string) => void
}

export function RevisionsTab({ revisions, onRestore }: Props) {
	return (
		<SectionCard title="Révisions" flush>
			{revisions.length === 0 ? (
				<Empty title="Aucune révision">
					Les modifications enregistrées dans cette session apparaissent ici.
				</Empty>
			) : null}
			<Rows>
				{revisions.map((revision, index) => (
					<div key={revision.label} className={ROW}>
						<span className="w-[54px]">
							<Tag>{revision.label}</Tag>
						</span>
						<Ttl
							title={`Modifié : ${revision.changed}`}
							sub={`${revision.author} · ${revision.when}`}
						/>
						{index === 0 ? (
							<Sev tone="ok">Actuelle</Sev>
						) : (
							<Button
								type="button"
								variant="outline"
								size="sm"
								aria-label={`Restaurer ${revision.label}`}
								onClick={() => onRestore(revision.label)}
							>
								Restaurer
							</Button>
						)}
					</div>
				))}
			</Rows>
		</SectionCard>
	)
}
