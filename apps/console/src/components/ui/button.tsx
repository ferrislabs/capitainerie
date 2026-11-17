import { cva, type VariantProps } from 'class-variance-authority'
import { Slot } from 'radix-ui'
import type * as React from 'react'

import { cn } from '#/lib/utils'

const buttonVariants = cva(
	"inline-flex shrink-0 items-center justify-center gap-2 rounded-md text-[13px] font-medium whitespace-nowrap transition-[color,background-color,border-color,box-shadow] duration-150 outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
	{
		variants: {
			variant: {
				default:
					'bg-primary text-primary-foreground hover:bg-primary/88',
				destructive:
					'bg-destructive text-white hover:bg-destructive/90 dark:text-primary-foreground',
				outline:
					'border border-input/50 bg-card text-foreground hover:bg-accent hover:text-accent-foreground',
				secondary:
					'bg-secondary text-secondary-foreground hover:bg-secondary/80',
				ghost:
					'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
				link: 'text-link underline-offset-4 hover:underline',
			},
			size: {
				default: 'h-7 px-3 has-[>svg]:px-2.5',
				sm: 'h-6 gap-1.5 rounded-md px-2 text-xs has-[>svg]:px-1.5',
				lg: 'h-8 rounded-md px-4 has-[>svg]:px-3',
				icon: 'size-7',
				'icon-sm': 'size-6',
			},
		},
		defaultVariants: {
			variant: 'default',
			size: 'default',
		},
	},
)

function Button({
	className,
	variant = 'default',
	size = 'default',
	asChild = false,
	...props
}: React.ComponentProps<'button'> &
	VariantProps<typeof buttonVariants> & {
		asChild?: boolean
	}) {
	const Comp = asChild ? Slot.Root : 'button'

	return (
		<Comp
			data-slot="button"
			data-variant={variant}
			data-size={size}
			className={cn(buttonVariants({ variant, size, className }))}
			{...props}
		/>
	)
}

export { Button, buttonVariants }
