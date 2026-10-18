import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

interface Props {
	title: ReactNode
	description?: ReactNode
	actions?: ReactNode
	flush?: boolean
	bare?: boolean
	className?: string
	children: ReactNode
	headingLevel?: 'h2' | 'h3'
}

/**
 * Une section de page : titre, description et actions posés sur le fond,
 * puis le contenu dans un cadre. `flush` retire la marge intérieure du cadre
 * (listes de lignes) ; `bare` retire le cadre (tableaux).
 */
export function SectionCard({
	title,
	description,
	actions,
	flush = false,
	bare = false,
	className,
	children,
	headingLevel: Heading = 'h2',
}: Props) {
	return (
		<section className={cn('flex min-w-0 flex-col gap-3', className)}>
			<div className="flex flex-wrap items-end justify-between gap-x-4 gap-y-2">
				<div className="min-w-0">
					<Heading className="text-[15px] leading-tight font-semibold">
						{title}
					</Heading>
					{description ? (
						<p className="mt-1 text-[13px] text-muted-foreground">
							{description}
						</p>
					) : null}
				</div>
				{actions ? (
					<div className="flex flex-wrap items-center gap-2">{actions}</div>
				) : null}
			</div>
			<div
				className={cn(
					'min-w-0',
					!bare &&
						'overflow-hidden rounded-lg border bg-card text-card-foreground',
					!bare && !flush && 'p-4',
				)}
			>
				{children}
			</div>
		</section>
	)
}
