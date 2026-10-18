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
				'mx-auto flex w-full max-w-[90rem] min-w-0 flex-col gap-5 px-4 py-5 sm:px-6 sm:py-6',
				className,
			)}
		>
			{children}
		</div>
	)
}
