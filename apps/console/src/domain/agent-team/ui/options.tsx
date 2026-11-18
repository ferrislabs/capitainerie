export interface OptionItem {
	value: string
	label: string
}

interface Props {
	values: ReadonlyArray<string | OptionItem>
}

export function Options({ values }: Props) {
	return values.map((item) => {
		const { value, label } =
			typeof item === 'string' ? { value: item, label: item } : item
		return (
			<option key={value} value={value}>
				{label}
			</option>
		)
	})
}
