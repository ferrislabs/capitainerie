export type DiffKind = 'same' | 'added' | 'removed'

export interface DiffLine {
	kind: DiffKind
	text: string
}

const MAX_CELLS = 4_000_000

function lines(text: string): Array<string> {
	return text === '' ? [] : text.split('\n')
}

function tagged(kind: DiffKind, items: ReadonlyArray<string>): Array<DiffLine> {
	return items.map((text) => ({ kind, text }))
}

function commonRun(
	a: ReadonlyArray<string>,
	b: ReadonlyArray<string>,
): Array<DiffLine> {
	const rows = a.length + 1
	const cols = b.length + 1
	const table = new Uint16Array(rows * cols)
	for (let i = a.length - 1; i >= 0; i--) {
		for (let j = b.length - 1; j >= 0; j--) {
			table[i * cols + j] =
				a[i] === b[j]
					? table[(i + 1) * cols + j + 1] + 1
					: Math.max(table[(i + 1) * cols + j], table[i * cols + j + 1])
		}
	}
	const diff: Array<DiffLine> = []
	let i = 0
	let j = 0
	while (i < a.length && j < b.length) {
		if (a[i] === b[j]) {
			diff.push({ kind: 'same', text: a[i] })
			i++
			j++
		} else if (table[(i + 1) * cols + j] >= table[i * cols + j + 1]) {
			diff.push({ kind: 'removed', text: a[i] })
			i++
		} else {
			diff.push({ kind: 'added', text: b[j] })
			j++
		}
	}
	return [
		...diff,
		...tagged('removed', a.slice(i)),
		...tagged('added', b.slice(j)),
	]
}

export function lineDiff(before: string, after: string): Array<DiffLine> {
	const a = lines(before)
	const b = lines(after)
	let start = 0
	while (start < a.length && start < b.length && a[start] === b[start]) start++
	let endA = a.length
	let endB = b.length
	while (endA > start && endB > start && a[endA - 1] === b[endB - 1]) {
		endA--
		endB--
	}
	const middleA = a.slice(start, endA)
	const middleB = b.slice(start, endB)
	const middle =
		(middleA.length + 1) * (middleB.length + 1) > MAX_CELLS
			? [...tagged('removed', middleA), ...tagged('added', middleB)]
			: commonRun(middleA, middleB)
	return [
		...tagged('same', a.slice(0, start)),
		...middle,
		...tagged('same', a.slice(endA)),
	]
}

export function hasChanges(diff: ReadonlyArray<DiffLine>): boolean {
	return diff.some((line) => line.kind !== 'same')
}
