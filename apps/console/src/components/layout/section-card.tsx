import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

interface Props {
	title: ReactNode
	description?: ReactNode
	actions?: ReactNode
	/** Pas de marge intérieure : listes de lignes et tableaux. */
	flush?: boolean
	/** Pas de cadre : le contenu se pose sur le fond. */
	bare?: boolean
	className?: string
	children: ReactNode
	headingLevel?: 'h2' | 'h3'
}

/** Panneau de la maquette : cadre fin, titre de 12 px dans un bandeau, actions à droite. */
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
		<section
			className={cn(
				'mb-2.5 min-w-0',
				!bare &&
					'overflow-hidden rounded-lg border bg-card text-card-foreground',
				className,
			)}
		>
			<header
				className={cn(
					'flex items-center gap-2 px-3 py-[9px]',
					!bare && 'border-b',
				)}
			>
				<Heading className="m-0 flex-1 text-xs leading-[1.4] font-semibold">
					{title}
				</Heading>
				{description ? (
					<span className="text-xs font-normal text-muted-foreground">
						{description}
					</span>
				) : null}
				{actions ? (
					<div className="flex items-center gap-2 text-xs text-primary">
						{actions}
					</div>
				) : null}
			</header>
			<div className={cn('min-w-0', !flush && 'p-3')}>{children}</div>
		</section>
	)
}
