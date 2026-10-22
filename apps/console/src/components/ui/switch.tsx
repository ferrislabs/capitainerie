import { Switch as SwitchPrimitive } from 'radix-ui'
import { type ComponentProps, type ReactNode, useId } from 'react'

import { ChoiceRow } from '#/components/ui/choice-row'
import { joinIds, useFieldControl } from '#/components/ui/field'
import { cn } from '#/lib/utils'

interface SwitchProps extends ComponentProps<typeof SwitchPrimitive.Root> {
	label?: ReactNode
	description?: ReactNode
}

function Switch({
	label,
	description,
	className,
	id,
	disabled,
	required,
	'aria-describedby': describedBy,
	'aria-invalid': ariaInvalid,
	...props
}: SwitchProps) {
	const control = useFieldControl({
		id,
		'aria-describedby': describedBy,
		'aria-invalid': ariaInvalid,
		required,
		disabled,
	})
	const generated = useId()
	const controlId = control.id ?? (label ? generated : undefined)
	const own = controlId && description ? `${controlId}-description` : undefined

	const toggle = (
		<SwitchPrimitive.Root
			data-slot="switch"
			id={controlId}
			disabled={control.disabled}
			required={required}
			aria-invalid={control.invalid || undefined}
			aria-describedby={joinIds(control.describedBy, own)}
			className={cn(
				'peer inline-flex h-5 w-9 shrink-0 items-center rounded-full border border-input bg-secondary p-0.5 transition-[background-color,border-color,box-shadow] duration-150 outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/25 disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive data-[state=checked]:border-primary data-[state=checked]:bg-primary',
				!label && className,
			)}
			{...props}
		>
			<SwitchPrimitive.Thumb className="pointer-events-none block size-3.5 rounded-full bg-muted-foreground transition-transform duration-150 data-[state=checked]:translate-x-4 data-[state=checked]:bg-primary-foreground" />
		</SwitchPrimitive.Root>
	)

	if (!label) return toggle

	return (
		<ChoiceRow
			control={<span className="flex">{toggle}</span>}
			id={controlId}
			label={label}
			description={description}
			disabled={control.disabled}
			className={className}
		/>
	)
}

export { Switch, type SwitchProps }
