import { Tooltip as TooltipPrimitive } from 'radix-ui'
import type * as React from 'react'

import { cn } from '#/lib/utils'

function TooltipProvider({
	delayDuration = 150,
	...props
}: React.ComponentProps<typeof TooltipPrimitive.Provider>) {
	return <TooltipPrimitive.Provider delayDuration={delayDuration} {...props} />
}

function Tooltip(props: React.ComponentProps<typeof TooltipPrimitive.Root>) {
	return <TooltipPrimitive.Root data-slot="tooltip" {...props} />
}

function TooltipTrigger(
	props: React.ComponentProps<typeof TooltipPrimitive.Trigger>,
) {
	return <TooltipPrimitive.Trigger data-slot="tooltip-trigger" {...props} />
}

function TooltipContent({
	className,
	sideOffset = 8,
	...props
}: React.ComponentProps<typeof TooltipPrimitive.Content>) {
	return (
		<TooltipPrimitive.Portal>
			<TooltipPrimitive.Content
				data-slot="tooltip-content"
				sideOffset={sideOffset}
				className={cn(
					'z-50 rounded-md bg-foreground px-2 py-1 text-xs font-medium text-background shadow-float',
					'animate-in fade-in-0 data-[state=closed]:animate-out data-[state=closed]:fade-out-0',
					className,
				)}
				{...props}
			/>
		</TooltipPrimitive.Portal>
	)
}

export { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger }
