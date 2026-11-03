import { RadioGroup as RadioGroupPrimitive } from 'radix-ui'
import { type ComponentProps, type ReactNode, useId } from 'react'

import { ChoiceRow } from '#/components/ui/choice-row'
import { useFieldControl } from '#/components/ui/field'
import { cn } from '#/lib/utils'

function RadioGroup({
	className,
	disabled,
	required,
	'aria-describedby': describedBy,
	'aria-invalid': ariaInvalid,
	'aria-labelledby': labelledBy,
	...props
}: ComponentProps<typeof RadioGroupPrimitive.Root>) {
	const control = useFieldControl({
		'aria-describedby': describedBy,
		'aria-invalid': ariaInvalid,
		required,
		disabled,
	})
	return (
		<RadioGroupPrimitive.Root
			data-slot="radio-group"
			disabled={control.disabled}
			required={required}
			aria-labelledby={labelledBy ?? control.labelId}
			aria-describedby={control.describedBy}
			aria-invalid={control.invalid || undefined}
			className={cn('grid gap-2', className)}
			{...props}
		/>
	)
}

interface RadioGroupItemProps
	extends ComponentProps<typeof RadioGroupPrimitive.Item> {
	label?: ReactNode
	description?: ReactNode
}

function RadioGroupItem({
	label,
	description,
	className,
	id,
	disabled,
	...props
}: RadioGroupItemProps) {
	const generated = useId()
	const itemId = id ?? generated
	const descriptionId = description ? `${itemId}-description` : undefined

	const radio = (
		<RadioGroupPrimitive.Item
			data-slot="radio-group-item"
			id={itemId}
			disabled={disabled}
			aria-describedby={descriptionId}
			className={cn(
				'peer flex size-4 shrink-0 items-center justify-center rounded-full border border-input bg-card transition-[color,box-shadow,border-color] duration-150 outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/25 disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:border-primary',
				!label && className,
			)}
			{...props}
		>
			<RadioGroupPrimitive.Indicator className="block size-2 rounded-full bg-primary" />
		</RadioGroupPrimitive.Item>
	)

	if (!label) return radio

	return (
		<ChoiceRow
			control={<span className="mt-0.5 flex">{radio}</span>}
			id={itemId}
			label={label}
			description={description}
			disabled={disabled}
			className={className}
		/>
	)
}

export { RadioGroup, RadioGroupItem, type RadioGroupItemProps }
