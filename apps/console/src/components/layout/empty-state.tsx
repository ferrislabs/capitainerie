import type { LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

interface Props {
	icon: LucideIcon
	title: string
	description: string
	action?: ReactNode
	className?: string
}

export function EmptyState({
	icon: Icon,
	title,
	description,
	action,
	className,
}: Props) {
	return (
		<div
			className={cn(
				'flex flex-col items-center gap-3 rounded-lg border border-dashed border-input/60 bg-card px-6 py-10 text-center',
				className,
			)}
		>
			<span className="flex size-10 items-center justify-center rounded-md bg-secondary text-secondary-foreground">
				<Icon className="size-5" aria-hidden="true" />
			</span>
			<div className="flex max-w-sm flex-col gap-1">
				<p className="text-sm font-semibold">{title}</p>
				<p className="text-[13px] text-muted-foreground">{description}</p>
			</div>
			{action}
		</div>
	)
}
