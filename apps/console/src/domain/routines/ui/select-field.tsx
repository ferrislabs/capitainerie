import { useId } from 'react'
import { Field } from '#/components/ui/field'
import { NativeSelect } from '#/components/ui/native-select'

interface Props {
	label: string
	value: string
	options: ReadonlyArray<string>
	onChange: (value: string) => void
	hint?: string
}

export function SelectField({ label, value, options, onChange, hint }: Props) {
	const id = useId()
	return (
		<Field label={label} id={id} hint={hint}>
			<NativeSelect
				id={id}
				value={value}
				onChange={(event) => onChange(event.target.value)}
			>
				{options.map((option) => (
					<option key={option} value={option}>
						{option}
					</option>
				))}
			</NativeSelect>
		</Field>
	)
}
