import { SectionCard } from '#/components/layout/section-card'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import type { Revision } from '#/domain/agent-team/agent-config'

interface Props {
	revisions: ReadonlyArray<Revision>
	onRestore: (label: string) => void
}

export function RevisionsTab({ revisions, onRestore }: Props) {
	return (
		<SectionCard title="Révisions" flush>
			<ul className="divide-y">
				{revisions.map((revision, index) => (
					<li
						key={revision.label}
						className="flex items-center gap-3 px-3 py-2.5"
					>
						<Badge variant="outline">{revision.label}</Badge>
						<span className="min-w-0 flex-1">
							<b className="block font-medium">Modifié : {revision.changed}</b>
							<span className="text-xs text-muted-foreground">
								{revision.author} · {revision.when}
							</span>
						</span>
						{index === 0 ? (
							<Badge variant="success">Actuelle</Badge>
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
					</li>
				))}
			</ul>
		</SectionCard>
	)
}
