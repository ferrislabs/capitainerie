import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

interface Props {
	children: ReactNode
	className?: string
}

export function Page({ children, className }: Props) {
	return (
		<div
			className={cn(
				'flex w-full max-w-[90rem] min-w-0 flex-col gap-6 px-4 pt-8 pb-10 sm:px-8 lg:pt-12',
				className,
			)}
		>
			{children}
		</div>
	)
}
