export function toggleValue(
	values: ReadonlyArray<string>,
	value: string,
): ReadonlyArray<string> {
	return values.includes(value)
		? values.filter((current) => current !== value)
		: [...values, value]
}
