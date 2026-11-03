import { TextInput, type TextInputProps } from '#/components/ui/text-input'
import { cn } from '#/lib/utils'

type DateInputProps = Omit<TextInputProps, 'type' | 'leading' | 'trailing'> & {
	min?: string
	max?: string
}

function DateInput({
	clearable = true,
	clearLabel = 'Effacer la date',
	className,
	...props
}: DateInputProps) {
	return (
		<TextInput
			{...props}
			type="date"
			clearable={clearable}
			clearLabel={clearLabel}
			className={cn(
				'figures [&::-webkit-calendar-picker-indicator]:cursor-pointer [&::-webkit-calendar-picker-indicator]:opacity-60 [&::-webkit-calendar-picker-indicator]:hover:opacity-100',
				className,
			)}
		/>
	)
}

export { DateInput, type DateInputProps }
