import { type ReactNode, useId } from 'react'
import { CTL, Fr } from '#/domain/sources/ui/kit'

export interface SelectOption<T extends string = string> {
	value: T
	label: string
}

interface Props<T extends string> {
	label: string
	value: T
	options: ReadonlyArray<SelectOption<T>>
	onChange: (value: T) => void
	hint?: ReactNode
	disabled?: boolean
}

export function SelectField<T extends string>({
	label,
	value,
	options,
	onChange,
	hint,
	disabled,
}: Props<T>) {
	const id = useId()
	return (
		<Fr label={<label htmlFor={id}>{label}</label>} hint={hint}>
			<select
				id={id}
				className={CTL}
				value={value}
				disabled={disabled}
				onChange={(event) => {
					const next = options.find((o) => o.value === event.target.value)
					if (next) onChange(next.value)
				}}
			>
				{options.map((option) => (
					<option key={option.value} value={option.value}>
						{option.label}
					</option>
				))}
			</select>
		</Fr>
	)
}
