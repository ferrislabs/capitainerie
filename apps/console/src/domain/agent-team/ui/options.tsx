interface Props {
	values: ReadonlyArray<string>
}

export function Options({ values }: Props) {
	return values.map((value) => (
		<option key={value} value={value}>
			{value}
		</option>
	))
}
