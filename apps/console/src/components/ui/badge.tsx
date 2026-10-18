import { cva, type VariantProps } from 'class-variance-authority'
import type * as React from 'react'

import { cn } from '#/lib/utils'

const badgeVariants = cva(
	'inline-flex w-fit shrink-0 items-center justify-center gap-1.5 overflow-hidden rounded-sm px-1.5 py-px text-xs leading-5 font-medium whitespace-nowrap figures',
	{
		variants: {
			variant: {
				default: 'bg-primary text-primary-foreground',
				secondary: 'bg-secondary text-secondary-foreground',
				muted: 'bg-muted text-muted-foreground',
				outline: 'border border-border text-muted-foreground',
				neutral: 'bg-signal-neutral-soft text-signal-neutral',
				running: 'bg-signal-running-soft text-signal-running',
				success: 'bg-signal-success-soft text-signal-success',
				danger: 'bg-signal-danger-soft text-signal-danger',
				destructive: 'bg-signal-danger-soft text-signal-danger',
				attention: 'bg-signal-attention-soft text-signal-attention-ink',
				signal:
					'bg-signal-attention text-signal-attention-foreground font-semibold',
			},
		},
		defaultVariants: {
			variant: 'default',
		},
	},
)

function Badge({
	className,
	variant,
	...props
}: React.ComponentProps<'span'> & VariantProps<typeof badgeVariants>) {
	return (
		<span
			data-slot="badge"
			data-variant={variant}
			className={cn(badgeVariants({ variant }), className)}
			{...props}
		/>
	)
}

export { Badge, badgeVariants }
