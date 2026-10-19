import type { ReactNode } from 'react'

export interface ComboboxOption {
	value: string
	label: string
	description?: string
	icon?: ReactNode
	disabled?: boolean
	keywords?: string
}

export interface ComboboxGroup {
	label: string
	options: ReadonlyArray<ComboboxOption>
}

export type ComboboxItem = ComboboxOption | ComboboxGroup

export interface ComboboxSection {
	label: string | undefined
	options: ReadonlyArray<ComboboxOption>
}

export function isGroup(item: ComboboxItem): item is ComboboxGroup {
	return 'options' in item
}

export function normalize(text: string): string {
	return text
		.normalize('NFD')
		.replace(/\p{Diacritic}/gu, '')
		.toLowerCase()
		.trim()
}

export function matchesQuery(option: ComboboxOption, query: string): boolean {
	const needle = normalize(query)
	if (needle === '') return true
	const haystack = normalize(`${option.label} ${option.keywords ?? ''}`)
	return needle.split(/\s+/).every((word) => haystack.includes(word))
}

export function buildSections(
	items: ReadonlyArray<ComboboxItem>,
	query: string,
): ComboboxSection[] {
	const sections: ComboboxSection[] = []
	let loose: ComboboxOption[] | undefined
	for (const item of items) {
		if (isGroup(item)) {
			loose = undefined
			const options = item.options.filter((option) =>
				matchesQuery(option, query),
			)
			if (options.length > 0) sections.push({ label: item.label, options })
		} else if (matchesQuery(item, query)) {
			if (!loose) {
				loose = []
				sections.push({ label: undefined, options: loose })
			}
			loose.push(item)
		}
	}
	return sections
}

export function flattenSections(
	sections: ReadonlyArray<ComboboxSection>,
): ComboboxOption[] {
	return sections.flatMap((section) => section.options)
}

export function allOptions(
	items: ReadonlyArray<ComboboxItem>,
): ComboboxOption[] {
	return items.flatMap((item) => (isGroup(item) ? item.options : [item]))
}

export function stepIndex(
	options: ReadonlyArray<ComboboxOption>,
	from: number,
	direction: 1 | -1,
): number {
	const count = options.length
	if (count === 0) return -1
	for (let step = 1; step <= count; step += 1) {
		const index = (((from + direction * step) % count) + count) % count
		if (options[index]?.disabled !== true) return index
	}
	return -1
}

export function edgeIndex(
	options: ReadonlyArray<ComboboxOption>,
	edge: 'first' | 'last',
): number {
	const indexes = options.map((_, index) => index)
	const ordered = edge === 'first' ? indexes : indexes.reverse()
	return ordered.find((index) => options[index]?.disabled !== true) ?? -1
}

export function typeaheadIndex(
	options: ReadonlyArray<ComboboxOption>,
	from: number,
	buffer: string,
): number {
	const needle = normalize(buffer)
	if (needle === '' || options.length === 0) return -1
	const repeated = [...needle].every((char) => char === needle[0])
	const prefix = repeated ? (needle[0] ?? needle) : needle
	const start = repeated || from < 0 ? from + 1 : from
	for (let step = 0; step < options.length; step += 1) {
		const index = (start + step) % options.length
		const option = options[index]
		if (
			option &&
			option.disabled !== true &&
			normalize(option.label).startsWith(prefix)
		) {
			return index
		}
	}
	return -1
}
