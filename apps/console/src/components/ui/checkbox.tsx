import { Check, Minus } from 'lucide-react'
import { Checkbox as CheckboxPrimitive } from 'radix-ui'
import { type ComponentProps, type ReactNode, useId } from 'react'

import { ChoiceRow } from '#/components/ui/choice-row'
import { joinIds, useFieldControl } from '#/components/ui/field'
import { cn } from '#/lib/utils'

interface CheckboxProps extends ComponentProps<typeof CheckboxPrimitive.Root> {
	label?: ReactNode
	description?: ReactNode
	invalid?: boolean
}

function Checkbox({
	label,
	description,
	invalid,
	className,
	id,
	disabled,
	required,
	'aria-describedby': describedBy,
	'aria-invalid': ariaInvalid,
	...props
}: CheckboxProps) {
	const control = useFieldControl({
		id,
		'aria-describedby': describedBy,
		'aria-invalid': ariaInvalid,
		required,
		disabled,
	})
	const generated = useId()
	const controlId = control.id ?? (label ? generated : undefined)
	const isInvalid = invalid === true || control.invalid
	const own = controlId && description ? `${controlId}-description` : undefined

	const box = (
		<CheckboxPrimitive.Root
			data-slot="checkbox"
			id={controlId}
			disabled={control.disabled}
			required={required}
			aria-required={control.required && !required ? true : undefined}
			aria-invalid={isInvalid || undefined}
			aria-describedby={joinIds(control.describedBy, own)}
			className={cn(
				'peer flex size-4 shrink-0 items-center justify-center rounded-sm border border-input bg-card text-primary-foreground transition-[color,box-shadow,border-color] duration-150 outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/25 disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 data-[state=checked]:border-primary data-[state=checked]:bg-primary data-[state=indeterminate]:border-primary data-[state=indeterminate]:bg-primary',
				!label && className,
			)}
			{...props}
		>
			<CheckboxPrimitive.Indicator>
				{props.checked === 'indeterminate' ? (
					<Minus aria-hidden="true" className="size-3" strokeWidth={3} />
				) : (
					<Check aria-hidden="true" className="size-3" strokeWidth={3} />
				)}
			</CheckboxPrimitive.Indicator>
		</CheckboxPrimitive.Root>
	)

	if (!label) return box

	return (
		<ChoiceRow
			control={<span className="mt-0.5 flex">{box}</span>}
			id={controlId}
			label={label}
			description={description}
			disabled={control.disabled}
			className={className}
		/>
	)
}

export { Checkbox, type CheckboxProps }
