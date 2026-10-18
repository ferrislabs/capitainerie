import { AlertDialog as AlertDialogPrimitive } from 'radix-ui'
import type * as React from 'react'

import { buttonVariants } from '#/components/ui/button'
import { cn } from '#/lib/utils'

function AlertDialog(
	props: React.ComponentProps<typeof AlertDialogPrimitive.Root>,
) {
	return <AlertDialogPrimitive.Root data-slot="alert-dialog" {...props} />
}

function AlertDialogContent({
	className,
	...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Content>) {
	return (
		<AlertDialogPrimitive.Portal>
			<AlertDialogPrimitive.Overlay
				data-slot="alert-dialog-overlay"
				className="fixed inset-0 z-50 bg-[oklch(0.2_0.04_258/0.45)] data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=closed]:animate-out data-[state=closed]:fade-out-0"
			/>
			<AlertDialogPrimitive.Content
				data-slot="alert-dialog-content"
				className={cn(
					'fixed top-1/2 left-1/2 z-50 flex w-[calc(100%-2rem)] max-w-md -translate-x-1/2 -translate-y-1/2 flex-col gap-4 rounded-lg border bg-popover p-5 text-popover-foreground shadow-float',
					'data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-[0.98] data-[state=closed]:animate-out data-[state=closed]:fade-out-0',
					className,
				)}
				{...props}
			/>
		</AlertDialogPrimitive.Portal>
	)
}

function AlertDialogHeader({
	className,
	...props
}: React.ComponentProps<'div'>) {
	return (
		<div
			data-slot="alert-dialog-header"
			className={cn('flex flex-col gap-1', className)}
			{...props}
		/>
	)
}

function AlertDialogFooter({
	className,
	...props
}: React.ComponentProps<'div'>) {
	return (
		<div
			data-slot="alert-dialog-footer"
			className={cn(
				'flex flex-col-reverse gap-2 border-t pt-4 sm:flex-row sm:justify-end',
				className,
			)}
			{...props}
		/>
	)
}

function AlertDialogTitle({
	className,
	...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Title>) {
	return (
		<AlertDialogPrimitive.Title
			data-slot="alert-dialog-title"
			className={cn('text-[15px] leading-tight font-semibold', className)}
			{...props}
		/>
	)
}

function AlertDialogDescription({
	className,
	...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Description>) {
	return (
		<AlertDialogPrimitive.Description
			data-slot="alert-dialog-description"
			className={cn('text-[13px] text-muted-foreground', className)}
			{...props}
		/>
	)
}

function AlertDialogAction({
	className,
	...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Action>) {
	return (
		<AlertDialogPrimitive.Action
			className={cn(buttonVariants({ variant: 'destructive' }), className)}
			{...props}
		/>
	)
}

function AlertDialogCancel({
	className,
	...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Cancel>) {
	return (
		<AlertDialogPrimitive.Cancel
			className={cn(buttonVariants({ variant: 'outline' }), className)}
			{...props}
		/>
	)
}

export {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
}
