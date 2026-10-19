interface SelectableStep {
	seq: number
	status: 'running' | 'succeeded' | 'failed' | 'skipped'
}

export function defaultSelectedSeq(
	steps: ReadonlyArray<SelectableStep>,
): number | undefined {
	const ordered = [...steps].sort((a, b) => a.seq - b.seq)
	const pick =
		ordered.find((step) => step.status === 'running') ??
		ordered.find((step) => step.status === 'failed') ??
		ordered[ordered.length - 1]
	return pick?.seq
}

export function moveSelection(
	seqs: ReadonlyArray<number>,
	current: number | undefined,
	key: string,
): number | undefined {
	if (seqs.length === 0) return undefined
	const last = seqs.length - 1
	const index = current === undefined ? -1 : seqs.indexOf(current)
	switch (key) {
		case 'ArrowDown':
			return seqs[Math.min(index + 1, last)]
		case 'ArrowUp':
			return seqs[Math.max(index - 1, 0)]
		case 'Home':
			return seqs[0]
		case 'End':
			return seqs[last]
		default:
			return undefined
	}
}
