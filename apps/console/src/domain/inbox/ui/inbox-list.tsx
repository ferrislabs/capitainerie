import { Link } from '@tanstack/react-router'
import type { InboxItem } from '#/domain/inbox/filter-inbox'
import { cn } from '#/lib/utils'

interface Props {
	items: ReadonlyArray<InboxItem>
}

const ROW =
	'flex min-h-9 w-full min-w-0 items-center gap-2.5 border-t px-3 py-1.5 first:border-t-0 hover:bg-secondary'

function Row({ item }: { item: InboxItem }) {
	const content = (
		<>
			<span
				role="img"
				aria-label={item.unread ? 'Non lu' : 'Lu'}
				className={cn(
					'w-[7px] flex-none',
					item.unread && 'h-[7px] rounded-full bg-primary',
				)}
			/>
			<span className="font-mono text-[11px] font-medium whitespace-nowrap text-faint">
				{item.ref || 'Routine'}
			</span>
			<span className="min-w-0 flex-1">
				<b className="block truncate font-medium leading-[19px]">
					{item.title}
				</b>
				<span className="block truncate text-xs leading-[1.4] text-muted-foreground">
					{item.source}
				</span>
			</span>
			<span className="font-mono text-[11.5px] whitespace-nowrap text-faint">
				{item.when}
			</span>
		</>
	)
	return item.ref ? (
		<Link to="/tasks/$taskId" params={{ taskId: item.id }} className={ROW}>
			{content}
		</Link>
	) : (
		<Link to="/routines" className={ROW}>
			{content}
		</Link>
	)
}

export function InboxList({ items }: Props) {
	return (
		<section className="overflow-hidden rounded-lg border bg-card">
			{items.length === 0 ? (
				<div className="px-4 py-[26px] text-center text-muted-foreground">
					<b className="mt-1.5 mb-0.5 block font-semibold text-foreground">
						Rien ici
					</b>
					Aucun élément pour ce filtre.
				</div>
			) : (
				<ul className="m-0 list-none p-0">
					{items.map((item) => (
						<li key={item.id}>
							<Row item={item} />
						</li>
					))}
				</ul>
			)}
		</section>
	)
}
