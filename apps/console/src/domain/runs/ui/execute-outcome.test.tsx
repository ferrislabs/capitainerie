import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { ExecuteOutput } from '#/domain/runs/execute-output'
import { ExecuteOutcome } from '#/domain/runs/ui/execute-outcome'

function output(overrides: Partial<ExecuteOutput> = {}): ExecuteOutput {
	return { noChanges: false, droppedToolCalls: 0, ...overrides }
}

describe('ExecuteOutcome', () => {
	it('links the draft pull request and shows the branch in mono', () => {
		render(
			<ExecuteOutcome
				output={output({
					pullRequest: 'https://github.com/acme/web/pull/7',
					branch: 'capitainerie/0123456789ab',
				})}
				failed={false}
				stepError={null}
			/>,
		)

		expect(
			screen
				.getByRole('link', { name: 'Pull request en brouillon' })
				.getAttribute('href'),
		).toBe('https://github.com/acme/web/pull/7')
		expect(screen.getByText('capitainerie/0123456789ab').className).toContain(
			'font-mono',
		)
	})

	it('says when nothing changed', () => {
		render(
			<ExecuteOutcome
				output={output({ noChanges: true })}
				failed={false}
				stepError={null}
			/>,
		)

		expect(screen.getByText('Aucun changement')).toBeTruthy()
		expect(screen.queryByRole('link')).toBeNull()
	})

	it('shows the failure reason only when the step does not already carry it', () => {
		const { rerender } = render(
			<ExecuteOutcome
				output={output({ error: 'le push a été refusé' })}
				failed
				stepError={null}
			/>,
		)
		expect(screen.getByRole('alert').textContent).toBe('le push a été refusé')

		rerender(
			<ExecuteOutcome
				output={output({ error: 'le push a été refusé' })}
				failed
				stepError="le push a été refusé"
			/>,
		)
		expect(screen.queryByRole('alert')).toBeNull()
	})

	it('counts the tool calls that were not kept', () => {
		const { rerender } = render(
			<ExecuteOutcome
				output={output({ droppedToolCalls: 12 })}
				failed={false}
				stepError={null}
			/>,
		)
		expect(screen.getByText("12 appels d'outils non affichés")).toBeTruthy()

		rerender(
			<ExecuteOutcome
				output={output({ droppedToolCalls: 1 })}
				failed={false}
				stepError={null}
			/>,
		)
		expect(screen.getByText("1 appel d'outil non affiché")).toBeTruthy()

		rerender(
			<ExecuteOutcome output={output()} failed={false} stepError={null} />,
		)
		expect(screen.queryByText(/non affiché/)).toBeNull()
	})
})
