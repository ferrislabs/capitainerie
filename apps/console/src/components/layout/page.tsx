import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

interface Props {
	children: ReactNode
	className?: string
}

/** Contenu d'une page : 14 px de marge, pleine largeur, comme la maquette. */
export function Page({ children, className }: Props) {
	return (
		<div
			className={cn(
				'flex w-full min-w-0 flex-col px-3.5 pt-3.5 pb-7',
				className,
			)}
		>
			{children}
		</div>
	)
}
