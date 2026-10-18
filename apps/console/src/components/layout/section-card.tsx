import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

interface Props {
	title: ReactNode
	description?: ReactNode
	actions?: ReactNode
	flush?: boolean
	className?: string
	children: ReactNode
	headingLevel?: 'h2' | 'h3'
}

export function SectionCard({
	title,
	description,
	actions,
	flush = false,
	className,
	children,
	headingLevel: Heading = 'h2',
}: Props) {
	return (
		<section
			className={cn(
				'flex min-w-0 flex-col overflow-hidden rounded-lg border bg-card text-card-foreground',
				className,
			)}
		>
			<div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-b px-4 py-3">
				<div className="min-w-0">
					<Heading className="text-[15px] leading-tight font-semibold">
						{title}
					</Heading>
					{description ? (
						<p className="mt-0.5 text-xs text-muted-foreground">
							{description}
						</p>
					) : null}
				</div>
				{actions ? (
					<div className="flex flex-wrap items-center gap-2">{actions}</div>
				) : null}
			</div>
			<div className={cn('min-w-0', !flush && 'p-4')}>{children}</div>
		</section>
	)
}
