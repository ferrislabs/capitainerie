import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { RunsView } from '#/domain/runs/pages/ui/runs-view'

describe('RunsView', () => {
	it('renders the heading and the empty state', () => {
		render(
			<RunsView
				heading="Runs"
				emptyTitle="Aucun run"
				emptyDescription="Rien pour l'instant."
			/>,
		)

		expect(screen.getByRole('heading', { name: 'Runs' })).toBeTruthy()
		expect(screen.getByText('Aucun run')).toBeTruthy()
		expect(screen.getByText("Rien pour l'instant.")).toBeTruthy()
	})
})
