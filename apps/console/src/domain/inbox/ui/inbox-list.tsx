import { GitMerge, ListChecks, Play, Repeat } from 'lucide-react'
import type { ComponentType } from 'react'
import { EmptyState } from '#/components/layout/empty-state'
import { SectionCard } from '#/components/layout/section-card'
import type { InboxItem, InboxKind } from '#/domain/inbox/filter-inbox'

interface Props {
	items: ReadonlyArray<InboxItem>
}

const KINDS: Record<
	InboxKind,
	{ label: string; icon: ComponentType<{ className?: string }> }
> = {
	task: { label: 'Tâche', icon: ListChecks },
	run: { label: 'Exécution', icon: Play },
	mr: { label: 'Merge request', icon: GitMerge },
	routine: { label: 'Routine', icon: Repeat },
}

export function InboxList({ items }: Props) {
	if (items.length === 0) {
		return (
			<EmptyState
				icon={ListChecks}
				title="Rien ici"
				description="Aucun élément pour ce filtre."
			/>
		)
	}
	return (
		<SectionCard title="Éléments" headingLevel="h2" flush>
			<ul className="divide-y">
				{items.map((item) => {
					const kind = KINDS[item.kind]
					const Icon = kind.icon
					return (
						<li key={item.id} className="flex items-center gap-2.5 px-3 py-2">
							<span
								role="img"
								aria-label={item.unread ? 'Non lu' : 'Lu'}
								className={
									item.unread
										? 'size-2 shrink-0 rounded-full bg-primary'
										: 'size-2 shrink-0'
								}
							/>
							<span role="img" aria-label={kind.label} className="shrink-0">
								<Icon className="size-4 text-muted-foreground" />
							</span>
							<span className="figures w-16 shrink-0 text-xs text-muted-foreground">
								{item.ref || 'Routine'}
							</span>
							<span className="min-w-0 flex-1">
								<b className="block truncate font-medium">{item.title}</b>
								<span className="block truncate text-xs text-muted-foreground">
									{item.source}
								</span>
							</span>
							<span className="shrink-0 text-xs text-muted-foreground">
								{item.when}
							</span>
						</li>
					)
				})}
			</ul>
		</SectionCard>
	)
}
