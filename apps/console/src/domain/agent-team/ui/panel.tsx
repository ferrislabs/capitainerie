import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

interface Props {
	children: ReactNode
	className?: string
}

export function Panel({ children, className }: Props) {
	return (
		<section
			className={cn(
				'mb-2.5 min-w-0 overflow-hidden rounded-lg border bg-card text-card-foreground',
				className,
			)}
		>
			{children}
		</section>
	)
}
