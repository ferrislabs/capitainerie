import { describe, expect, it } from 'vitest'
import { defaultSelectedSeq, moveSelection } from '#/domain/runs/step-selection'

type Status = 'running' | 'succeeded' | 'failed' | 'skipped'

function steps(...statuses: Status[]) {
	return statuses.map((status, seq) => ({ seq, status }))
}

describe('defaultSelectedSeq', () => {
	it('opens on the running step', () => {
		expect(defaultSelectedSeq(steps('succeeded', 'running', 'skipped'))).toBe(1)
	})

	it('opens on the failed step when nothing runs', () => {
		expect(defaultSelectedSeq(steps('succeeded', 'failed', 'skipped'))).toBe(1)
	})

	it('opens on the last step otherwise', () => {
		expect(defaultSelectedSeq(steps('succeeded', 'succeeded'))).toBe(1)
	})

	it('has nothing to select without steps', () => {
		expect(defaultSelectedSeq([])).toBeUndefined()
	})
})

describe('moveSelection', () => {
	const seqs = [0, 1, 3]

	it('moves down and up through the sequence numbers', () => {
		expect(moveSelection(seqs, 1, 'ArrowDown')).toBe(3)
		expect(moveSelection(seqs, 1, 'ArrowUp')).toBe(0)
	})

	it('stops at both ends', () => {
		expect(moveSelection(seqs, 3, 'ArrowDown')).toBe(3)
		expect(moveSelection(seqs, 0, 'ArrowUp')).toBe(0)
	})

	it('jumps to the ends with Home and End', () => {
		expect(moveSelection(seqs, 1, 'Home')).toBe(0)
		expect(moveSelection(seqs, 1, 'End')).toBe(3)
	})

	it('starts from the first step when nothing is selected', () => {
		expect(moveSelection(seqs, undefined, 'ArrowDown')).toBe(0)
	})

	it('ignores other keys', () => {
		expect(moveSelection(seqs, 1, 'Enter')).toBeUndefined()
		expect(moveSelection([], 1, 'ArrowDown')).toBeUndefined()
	})
})
