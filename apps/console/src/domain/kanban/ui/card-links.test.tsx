import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { CardLinks } from '#/domain/kanban/ui/card-links'

const CARD = {
	id: 'c1',
	version: 3,
	client: { id: 'acme', name: 'Acme' },
	project: { id: 'p1', name: 'Site' },
} as Schemas.CardResponse

const CLIENTS = [
	{ id: 'acme', name: 'Acme' },
	{ id: 'globex', name: 'Globex' },
] as Schemas.ClientResponse[]

const PROJECTS = [
	{ id: 'p1', name: 'Site', client: { id: 'acme', name: 'Acme' } },
	{ id: 'p2', name: 'Refonte', client: { id: 'acme', name: 'Acme' } },
	{ id: 'p3', name: 'App', client: { id: 'globex', name: 'Globex' } },
] as Schemas.ProjectResponse[]

function setup(errorMessage?: string) {
	const onLink = vi.fn()
	render(
		<CardLinks
			card={CARD}
			clients={CLIENTS}
			projects={PROJECTS}
			isPending={false}
			errorMessage={errorMessage}
			onLink={onLink}
		/>,
	)
	return onLink
}

describe('CardLinks', () => {
	it('lists only the projects of the card’s client', () => {
		setup()

		const options = screen
			.getAllByRole('option')
			.map((option) => option.textContent)
		expect(options).toContain('Refonte')
		expect(options).not.toContain('App')
	})

	it('changes the project alone', async () => {
		const onLink = setup()

		await userEvent.selectOptions(screen.getByLabelText('Projet'), 'p2')

		expect(onLink).toHaveBeenCalledWith({ project_id: 'p2' })
	})

	it('drops the project with another client', async () => {
		const onLink = setup()

		await userEvent.selectOptions(screen.getByLabelText('Client'), 'globex')

		expect(onLink).toHaveBeenCalledWith({
			client_id: 'globex',
			project_id: null,
		})
	})

	it('clears with the empty choice', async () => {
		const onLink = setup()

		await userEvent.selectOptions(screen.getByLabelText('Projet'), '')

		expect(onLink).toHaveBeenCalledWith({ project_id: null })
	})

	it('shows the reload message of a conflict', () => {
		setup('La carte a changé, rechargée.')

		expect(screen.getByText('La carte a changé, rechargée.')).toBeTruthy()
	})
})
