import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

interface Props {
	children: ReactNode
	className?: string
}

export const CARD =
	'min-w-0 overflow-hidden rounded-xl border bg-card text-card-foreground'

export function Panel({ children, className }: Props) {
	return <section className={cn('mb-2.5', CARD, className)}>{children}</section>
}

interface SectionProps {
	title: ReactNode
	description?: ReactNode
	actions?: ReactNode
	/** Sans marge intérieure : listes de lignes. */
	flush?: boolean
	className?: string
	children: ReactNode
}

/** Section de la fiche agent : titre posé au-dessus d'une carte aérée. */
export function DetailSection({
	title,
	description,
	actions,
	flush = false,
	className,
	children,
}: SectionProps) {
	return (
		<section className={cn('flex min-w-0 flex-col gap-3', className)}>
			<header className="flex min-h-7 flex-wrap items-center gap-x-4 gap-y-1">
				<h2 className="m-0 flex-1 text-[17px] leading-snug font-semibold tracking-[-0.01em]">
					{title}
				</h2>
				{actions ? (
					<div className="flex items-center gap-3 text-[13px] text-muted-foreground [&_a:hover]:text-foreground">
						{actions}
					</div>
				) : null}
				{description ? (
					<p className="m-0 basis-full text-muted-foreground">{description}</p>
				) : null}
			</header>
			<div className={cn(CARD, !flush && 'p-6')}>{children}</div>
		</section>
	)
}

/** Grille de deux sections, une seule colonne sous 900 px. */
export function SectionGrid({ children }: { children: ReactNode }) {
	return (
		<div className="grid items-start gap-x-6 gap-y-10 min-[900px]:grid-cols-2">
			{children}
		</div>
	)
}
