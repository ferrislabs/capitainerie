import { useId } from 'react'
import { CTL, Fr } from '#/domain/sources/ui/mockup'

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
		<Fr label={<label htmlFor={id}>{label}</label>} hint={hint}>
			<select
				id={id}
				className={CTL}
				value={value}
				onChange={(event) => onChange(event.target.value)}
			>
				{options.map((option) => (
					<option key={option} value={option}>
						{option}
					</option>
				))}
			</select>
		</Fr>
	)
}
