import type { ReactNode } from 'react'

interface Props {
	title: ReactNode
	description?: ReactNode
	actions?: ReactNode
	leading?: ReactNode
	meta?: ReactNode
}

export function PageHeader({
	title,
	description,
	actions,
	leading,
	meta,
}: Props) {
	return (
		<header className="flex flex-col gap-3 border-b pb-5">
			{leading}
			<div className="flex flex-wrap items-start justify-between gap-x-6 gap-y-3">
				<div className="flex min-w-0 flex-col gap-1">
					<div className="flex flex-wrap items-center gap-x-3 gap-y-1">
						<h1 className="text-[1.375rem] leading-tight font-semibold tracking-[-0.015em]">
							{title}
						</h1>
						{meta}
					</div>
					{description ? (
						<p className="max-w-[70ch] text-[13px] text-muted-foreground">
							{description}
						</p>
					) : null}
				</div>
				{actions ? (
					<div className="flex shrink-0 flex-wrap items-center gap-2">
						{actions}
					</div>
				) : null}
			</div>
		</header>
	)
}
