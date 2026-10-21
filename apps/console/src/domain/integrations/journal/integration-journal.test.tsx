import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { IntegrationJournal } from '#/domain/integrations/journal/integration-journal'

type Action = Schemas.IntegrationActionResponse

function minutesAgo(minutes: number): string {
	return new Date(Date.now() - minutes * 60_000).toISOString()
}

function action(overrides: Partial<Action> = {}): Action {
	return {
		id: 'a1',
		run_id: 'r1',
		card_id: 'c1',
		instance_id: 'i1',
		instance_name: 'Odoo prod',
		instance_environment: 'production',
		instance_kind: 'odoo_json2',
		actor: 'capitainerie-agent-integrator',
		tool: 'odoo_write',
		access: 'write',
		target: { model: 'res.partner', ids: [2] },
		arguments: { model: 'res.partner', ids: [2], values: { name: 'Acme' } },
		outcome: 'succeeded',
		revertible: true,
		created_at: minutesAgo(5),
		...overrides,
	}
}

function rows() {
	return screen.getAllByRole('listitem')
}

describe('IntegrationJournal', () => {
	it('lists the actions oldest first with instance, environment, tool, target and outcome', () => {
		render(
			<IntegrationJournal
				actions={[
					action({
						id: 'a2',
						instance_name: 'Flux',
						instance_environment: 'staging',
						instance_kind: 'n8n',
						tool: 'n8n_update_workflow',
						target: { workflow_id: 'wf1' },
						created_at: minutesAgo(1),
					}),
					action({ id: 'a1', created_at: minutesAgo(5) }),
				]}
			/>,
		)

		const [first, second] = rows()
		expect(within(first).getByText('Odoo prod')).toBeTruthy()
		expect(within(first).getByText('Production')).toBeTruthy()
		expect(within(first).getByText('Odoo · modifier')).toBeTruthy()
		expect(within(first).getByText('res.partner #2')).toBeTruthy()
		expect(within(first).getByText('Réussie')).toBeTruthy()
		expect(within(first).getByText('il y a 5 minutes')).toBeTruthy()
		expect(within(second).getByText('Flux')).toBeTruthy()
		expect(within(second).getByText('Staging')).toBeTruthy()
		expect(within(second).getByText('n8n · modifier un workflow')).toBeTruthy()
		expect(within(second).getByText('workflow wf1')).toBeTruthy()
	})

	it('explains a refused action in French', () => {
		render(
			<IntegrationJournal
				actions={[
					action({
						outcome: 'denied',
						reason_code: 'write_not_granted',
						reason: 'this instance is granted read-only to the intervention',
					}),
				]}
			/>,
		)

		const [row] = rows()
		expect(within(row).getByText('Refusée')).toBeTruthy()
		expect(
			within(row).getByText('écriture non autorisée sur cette instance'),
		).toBeTruthy()
		expect(within(row).queryByText(/granted read-only/)).toBeNull()
	})

	it('explains a failed action and keeps an unknown reason as is', () => {
		render(
			<IntegrationJournal
				actions={[
					action({
						id: 'a1',
						outcome: 'failed',
						reason_code: 'before_unreadable',
						reason: 'could not read',
					}),
					action({
						id: 'a2',
						outcome: 'failed',
						reason_code: 'brand_new',
						reason: 'the instance exploded',
						created_at: minutesAgo(2),
					}),
				]}
			/>,
		)

		const [first, second] = rows()
		expect(within(first).getByText('En échec')).toBeTruthy()
		expect(
			within(first).getByText(
				'l’état d’avant n’a pas pu être lu : rien n’a été écrit',
			),
		).toBeTruthy()
		expect(within(second).getByText('the instance exploded')).toBeTruthy()
	})

	it('shows a pending action as in progress', () => {
		render(<IntegrationJournal actions={[action({ outcome: 'pending' })]} />)

		expect(within(rows()[0]).getByText('En cours')).toBeTruthy()
	})

	it('opens a row on its arguments, before, after and their diff', async () => {
		const user = userEvent.setup()
		render(
			<IntegrationJournal
				actions={[
					action({
						before: { name: 'Acme', email: '[masqué]' },
						after: { name: 'Acme SA', email: '[masqué]' },
					}),
				]}
			/>,
		)
		const toggle = screen.getByRole('button', { name: /Odoo · modifier/ })
		expect(toggle.getAttribute('aria-expanded')).toBe('false')
		expect(screen.queryByText('Avant')).toBeNull()

		await user.click(toggle)

		expect(toggle.getAttribute('aria-expanded')).toBe('true')
		const args = screen.getByRole('region', { name: 'Arguments' })
		expect(args.textContent).toContain('"values": {')
		expect(args.textContent).toContain('"name": "Acme"')
		const before = screen.getByRole('region', { name: 'Avant' })
		expect(before.textContent).toContain('"name": "Acme"')
		expect(before.textContent).toContain('"email": "[masqué]"')
		const after = screen.getByRole('region', { name: 'Après' })
		expect(after.textContent).toContain('"name": "Acme SA"')
		const diff = screen.getByRole('region', { name: 'Différence' })
		const kinds = [...diff.querySelectorAll('[data-kind]')].map((line) => [
			line.getAttribute('data-kind'),
			line.textContent,
		])
		expect(kinds).toEqual([
			['same', '  {'],
			['removed', '-   "name": "Acme",'],
			['added', '+   "name": "Acme SA",'],
			['same', '    "email": "[masqué]"'],
			['same', '  }'],
		])

		await user.click(toggle)
		expect(screen.queryByRole('region', { name: 'Avant' })).toBeNull()
	})

	it('has no diff when there is no state before or after', async () => {
		const user = userEvent.setup()
		render(
			<IntegrationJournal
				actions={[
					action({
						tool: 'odoo_create',
						revertible: false,
						before: null,
						after: { id: 9 },
					}),
				]}
			/>,
		)

		await user.click(screen.getByRole('button', { name: /Odoo · créer/ }))

		expect(
			within(screen.getByRole('region', { name: 'Avant' })).getByText('—'),
		).toBeTruthy()
		expect(screen.getByRole('region', { name: 'Après' })).toBeTruthy()
		expect(screen.queryByRole('region', { name: 'Différence' })).toBeNull()
	})

	it('says when nothing changed', async () => {
		const user = userEvent.setup()
		render(
			<IntegrationJournal
				actions={[action({ before: { a: 1 }, after: { a: 1 } })]}
			/>,
		)

		await user.click(screen.getByRole('button', { name: /Odoo · modifier/ }))

		expect(
			within(screen.getByRole('region', { name: 'Différence' })).getByText(
				'Aucune différence.',
			),
		).toBeTruthy()
	})

	it('links a restoration to the action it undoes, and flags the restored one', () => {
		const original = action({
			id: 'a1',
			created_at: '2026-10-20T14:03:00Z',
			reverted_by_action_id: 'a2',
		})
		const restoration = action({
			id: 'a2',
			created_at: minutesAgo(1),
			reverts_action_id: 'a1',
		})
		const orphan = action({
			id: 'a3',
			created_at: minutesAgo(1),
			reverts_action_id: 'elsewhere',
		})
		render(<IntegrationJournal actions={[original, restoration, orphan]} />)

		const when = new Intl.DateTimeFormat('fr-FR', {
			dateStyle: 'short',
			timeStyle: 'short',
		}).format(new Date('2026-10-20T14:03:00Z'))
		const [first, second, third] = rows()
		expect(within(first).getByText('Restaurée')).toBeTruthy()
		expect(
			within(second).getByText(`Restauration de l’action du ${when}`),
		).toBeTruthy()
		expect(within(second).queryByText('Restaurée')).toBeNull()
		expect(
			within(third).getByText('Restauration d’une autre action'),
		).toBeTruthy()
	})
})
