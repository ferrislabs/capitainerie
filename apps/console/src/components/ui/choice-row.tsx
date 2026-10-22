import type { ReactNode } from 'react'

import { cn } from '#/lib/utils'

interface ChoiceRowProps {
	control: ReactNode
	id: string | undefined
	label: ReactNode
	description?: ReactNode
	disabled?: boolean
	className?: string
}

function ChoiceRow({
	control,
	id,
	label,
	description,
	disabled,
	className,
}: ChoiceRowProps) {
	const descriptionId = id && description ? `${id}-description` : undefined
	return (
		<div className={cn('flex items-start gap-2', className)}>
			{control}
			<div className="flex min-w-0 flex-col gap-0.5">
				<label
					htmlFor={id}
					className={cn(
						'text-sm leading-5 select-none',
						disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer',
					)}
				>
					{label}
				</label>
				{description ? (
					<span id={descriptionId} className="text-xs text-muted-foreground">
						{description}
					</span>
				) : null}
			</div>
		</div>
	)
}

export { ChoiceRow }
