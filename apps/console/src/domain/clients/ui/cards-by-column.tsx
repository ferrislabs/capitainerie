import { Link } from '@tanstack/react-router'
import type { Schemas } from '#/api/api.client'
import { COLUMN_LABELS, COLUMNS } from '#/domain/kanban/columns'

const SHOWN = 5

interface Props {
	counts: ReadonlyArray<Schemas.ColumnCountResponse>
	cards: ReadonlyArray<Schemas.CardResponse>
}

export function CardsByColumn({ counts, cards }: Props) {
	return (
		<div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
			{COLUMNS.map((column) => {
				const count = counts.find((item) => item.column === column)?.cards ?? 0
				const items = cards.filter((card) => card.column === column)
				return (
					<section
						key={column}
						aria-label={COLUMN_LABELS[column]}
						className="flex min-w-0 flex-col gap-2 rounded-lg border bg-card p-4"
					>
						<h3 className="text-[13px] font-medium text-muted-foreground">
							{COLUMN_LABELS[column]}
						</h3>
						<p className="figures text-2xl leading-tight font-semibold tracking-[-0.02em]">
							{count}
						</p>
						<ul className="flex flex-col gap-1">
							{items.slice(0, SHOWN).map((card) => (
								<li key={card.id} className="min-w-0">
									<Link
										to="/kanban"
										search={{ card: card.id }}
										className="block truncate text-[13px] hover:text-link hover:underline"
									>
										{card.title}
									</Link>
								</li>
							))}
							{items.length > SHOWN ? (
								<li className="text-xs text-muted-foreground">
									+ {items.length - SHOWN} autres
								</li>
							) : null}
						</ul>
					</section>
				)
			})}
		</div>
	)
}
