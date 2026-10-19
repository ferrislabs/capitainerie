import { Check, ChevronRight } from 'lucide-react'
import { DropdownMenu as DropdownMenuPrimitive } from 'radix-ui'
import type * as React from 'react'
import { useId } from 'react'

import { cn } from '#/lib/utils'

function DropdownMenu(
	props: React.ComponentProps<typeof DropdownMenuPrimitive.Root>,
) {
	return <DropdownMenuPrimitive.Root data-slot="dropdown-menu" {...props} />
}

function DropdownMenuTrigger(
	props: React.ComponentProps<typeof DropdownMenuPrimitive.Trigger>,
) {
	return (
		<DropdownMenuPrimitive.Trigger
			data-slot="dropdown-menu-trigger"
			{...props}
		/>
	)
}

function DropdownMenuContent({
	className,
	sideOffset = 6,
	...props
}: React.ComponentProps<typeof DropdownMenuPrimitive.Content>) {
	return (
		<DropdownMenuPrimitive.Portal>
			<DropdownMenuPrimitive.Content
				data-slot="dropdown-menu-content"
				sideOffset={sideOffset}
				className={cn(
					'z-50 min-w-48 overflow-hidden rounded-lg border bg-popover p-1 text-popover-foreground shadow-float',
					'data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-[0.98] data-[state=closed]:animate-out data-[state=closed]:fade-out-0',
					className,
				)}
				{...props}
			/>
		</DropdownMenuPrimitive.Portal>
	)
}

const ITEM_BASE =
	"relative flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-sm outline-none select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4"

const ITEM_TONES = {
	default:
		'data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground',
	destructive:
		'text-signal-danger data-[highlighted]:bg-signal-danger-soft data-[highlighted]:text-signal-danger [&_svg]:text-signal-danger',
} as const

function DropdownMenuShortcut({
	className,
	...props
}: React.ComponentProps<'kbd'>) {
	return (
		<kbd
			data-slot="dropdown-menu-shortcut"
			className={cn(
				'figures ml-auto pl-4 font-sans text-xs tracking-wide text-muted-foreground',
				className,
			)}
			{...props}
		/>
	)
}

function DropdownMenuItem({
	className,
	variant = 'default',
	shortcut,
	children,
	...props
}: React.ComponentProps<typeof DropdownMenuPrimitive.Item> & {
	variant?: keyof typeof ITEM_TONES
	shortcut?: string
}) {
	return (
		<DropdownMenuPrimitive.Item
			data-slot="dropdown-menu-item"
			data-variant={variant}
			className={cn(ITEM_BASE, ITEM_TONES[variant], className)}
			{...props}
		>
			{children}
			{shortcut ? (
				<DropdownMenuShortcut>{shortcut}</DropdownMenuShortcut>
			) : null}
		</DropdownMenuPrimitive.Item>
	)
}

function DropdownMenuGroup(
	props: React.ComponentProps<typeof DropdownMenuPrimitive.Group>,
) {
	return (
		<DropdownMenuPrimitive.Group data-slot="dropdown-menu-group" {...props} />
	)
}

function DropdownMenuSection({
	label,
	children,
	className,
}: {
	label: string
	children: React.ReactNode
	className?: string
}) {
	const labelId = useId()
	return (
		<DropdownMenuPrimitive.Group
			data-slot="dropdown-menu-section"
			aria-labelledby={labelId}
			className={className}
		>
			<DropdownMenuPrimitive.Label
				id={labelId}
				className="px-2 pt-1.5 pb-1 text-xs font-medium text-muted-foreground"
			>
				{label}
			</DropdownMenuPrimitive.Label>
			{children}
		</DropdownMenuPrimitive.Group>
	)
}

function DropdownMenuSub(
	props: React.ComponentProps<typeof DropdownMenuPrimitive.Sub>,
) {
	return <DropdownMenuPrimitive.Sub data-slot="dropdown-menu-sub" {...props} />
}

function DropdownMenuSubTrigger({
	className,
	children,
	...props
}: React.ComponentProps<typeof DropdownMenuPrimitive.SubTrigger>) {
	return (
		<DropdownMenuPrimitive.SubTrigger
			data-slot="dropdown-menu-sub-trigger"
			className={cn(
				ITEM_BASE,
				ITEM_TONES.default,
				'data-[state=open]:bg-accent data-[state=open]:text-accent-foreground',
				className,
			)}
			{...props}
		>
			{children}
			<ChevronRight
				aria-hidden="true"
				className="ml-auto size-4 text-muted-foreground"
			/>
		</DropdownMenuPrimitive.SubTrigger>
	)
}

function DropdownMenuSubContent({
	className,
	...props
}: React.ComponentProps<typeof DropdownMenuPrimitive.SubContent>) {
	return (
		<DropdownMenuPrimitive.Portal>
			<DropdownMenuPrimitive.SubContent
				data-slot="dropdown-menu-sub-content"
				className={cn(
					'z-50 min-w-44 overflow-hidden rounded-lg border bg-popover p-1 text-popover-foreground shadow-float',
					'data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-[0.98] data-[state=closed]:animate-out data-[state=closed]:fade-out-0',
					className,
				)}
				{...props}
			/>
		</DropdownMenuPrimitive.Portal>
	)
}

function DropdownMenuLabel({
	className,
	...props
}: React.ComponentProps<typeof DropdownMenuPrimitive.Label>) {
	return (
		<DropdownMenuPrimitive.Label
			data-slot="dropdown-menu-label"
			className={cn('px-2 py-1.5', className)}
			{...props}
		/>
	)
}

function DropdownMenuSeparator({
	className,
	...props
}: React.ComponentProps<typeof DropdownMenuPrimitive.Separator>) {
	return (
		<DropdownMenuPrimitive.Separator
			data-slot="dropdown-menu-separator"
			className={cn('-mx-1 my-1 h-px bg-border', className)}
			{...props}
		/>
	)
}

function DropdownMenuRadioGroup(
	props: React.ComponentProps<typeof DropdownMenuPrimitive.RadioGroup>,
) {
	return (
		<DropdownMenuPrimitive.RadioGroup
			data-slot="dropdown-menu-radio-group"
			{...props}
		/>
	)
}

function DropdownMenuRadioItem({
	className,
	children,
	...props
}: React.ComponentProps<typeof DropdownMenuPrimitive.RadioItem>) {
	return (
		<DropdownMenuPrimitive.RadioItem
			data-slot="dropdown-menu-radio-item"
			className={cn(
				"relative flex cursor-default items-center gap-2 rounded-md py-1.5 pr-8 pl-2 text-sm outline-none select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
				className,
			)}
			{...props}
		>
			{children}
			<span className="absolute right-2 flex size-4 items-center justify-center">
				<DropdownMenuPrimitive.ItemIndicator>
					<Check aria-hidden="true" />
				</DropdownMenuPrimitive.ItemIndicator>
			</span>
		</DropdownMenuPrimitive.RadioItem>
	)
}

function DropdownMenuCheckboxItem({
	className,
	children,
	...props
}: React.ComponentProps<typeof DropdownMenuPrimitive.CheckboxItem>) {
	return (
		<DropdownMenuPrimitive.CheckboxItem
			data-slot="dropdown-menu-checkbox-item"
			className={cn(
				"relative flex cursor-default items-center gap-2 rounded-md py-1.5 pr-8 pl-2 text-sm outline-none select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
				className,
			)}
			{...props}
		>
			{children}
			<span className="absolute right-2 flex size-4 items-center justify-center">
				<DropdownMenuPrimitive.ItemIndicator>
					<Check aria-hidden="true" />
				</DropdownMenuPrimitive.ItemIndicator>
			</span>
		</DropdownMenuPrimitive.CheckboxItem>
	)
}

export {
	DropdownMenuGroup,
	DropdownMenuSection,
	DropdownMenuShortcut,
	DropdownMenuSub,
	DropdownMenuSubContent,
	DropdownMenuSubTrigger,
	DropdownMenuCheckboxItem,
	DropdownMenuRadioGroup,
	DropdownMenuRadioItem,
	DropdownMenu,
	DropdownMenuTrigger,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
}
