import { Link } from '@tanstack/react-router'
import type { LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

export type CardTarget =
	| '/runs'
	| '/live'
	| '/kanban'
	| '/agents'
	| '/settings/providers'
	| '/settings/models'
	| '/settings/integrations'

interface Props {
	to: CardTarget
	icon: LucideIcon
	title: string
	value?: ReactNode
	hint?: ReactNode
	className?: string
}

export function LinkCard({
	to,
	icon: Icon,
	title,
	value,
	hint,
	className,
}: Props) {
	return (
		<Link
			to={to}
			className={cn(
				'flex min-w-0 flex-col gap-2 rounded-lg border bg-card p-4 text-card-foreground transition-colors duration-150',
				'hover:bg-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring',
				className,
			)}
		>
			<span className="flex items-center gap-2 text-[13px] font-medium text-muted-foreground">
				<Icon aria-hidden="true" className="size-4 shrink-0" />
				{title}
			</span>
			{value !== undefined ? (
				<span className="figures flex min-h-8 items-center text-2xl leading-tight font-semibold tracking-[-0.02em]">
					{value}
				</span>
			) : null}
			{hint ? (
				<span className="text-[13px] text-muted-foreground">{hint}</span>
			) : null}
		</Link>
	)
}
