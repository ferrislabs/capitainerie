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
				'flex w-full max-w-[90rem] min-w-0 flex-col gap-4 px-4 pt-5 pb-8 sm:px-6',
				className,
			)}
		>
			{children}
		</div>
	)
}
