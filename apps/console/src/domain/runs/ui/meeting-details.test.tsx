import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { MeetingDetails } from '#/domain/runs/ui/meeting-details'

function setup(overrides: Partial<Parameters<typeof MeetingDetails>[0]> = {}) {
	const props = {
		meetingDate: '',
		participants: ['Alice', 'Bob'],
		participantDraft: '',
		repository: '',
		disabled: false,
		onMeetingDateChange: vi.fn(),
		onParticipantDraftChange: vi.fn(),
		onParticipantDraftCommit: vi.fn(),
		onParticipantRemove: vi.fn(),
		onRepositoryChange: vi.fn(),
		...overrides,
	}
	render(<MeetingDetails {...props} />)
	return props
}

describe('MeetingDetails', () => {
	it('is a collapsed section', () => {
		setup()

		const details = screen.getByText('Détails de la réunion').closest('details')
		expect(details?.open).toBe(false)
	})

	it('commits the draft on Enter and removes a tag', async () => {
		const props = setup()
		const user = userEvent.setup()

		await user.type(screen.getByLabelText('Participants'), '{Enter}')
		await user.click(screen.getByRole('button', { name: 'Retirer Alice' }))

		expect(props.onParticipantDraftCommit).toHaveBeenCalled()
		expect(props.onParticipantRemove).toHaveBeenCalledWith('Alice')
	})

	it('removes the last tag on Backspace in an empty field', async () => {
		const props = setup()
		const user = userEvent.setup()

		await user.type(screen.getByLabelText('Participants'), '{Backspace}')

		expect(props.onParticipantRemove).toHaveBeenCalledWith('Bob')
	})

	it('offers the installation repositories when there are some', async () => {
		const props = setup({
			repositories: [{ full_name: 'acme/web', name: 'web', owner: 'acme' }],
		})
		const user = userEvent.setup()

		await user.selectOptions(
			screen.getByLabelText('Dépôt par défaut'),
			'acme/web',
		)

		expect(props.onRepositoryChange).toHaveBeenCalledWith('acme/web')
	})

	it('falls back to a text field without installation', async () => {
		const props = setup({
			repositoryError: 'Le dépôt doit être au format propriétaire/nom.',
		})
		const user = userEvent.setup()

		await user.type(screen.getByLabelText('Dépôt par défaut'), 'a')

		expect(props.onRepositoryChange).toHaveBeenCalledWith('a')
		expect(screen.getByRole('alert').textContent).toMatch(/propriétaire\/nom/)
	})

	it('forwards the date', async () => {
		const props = setup()
		const user = userEvent.setup()

		await user.type(screen.getByLabelText('Date de la réunion'), '2026-10-19')

		expect(props.onMeetingDateChange).toHaveBeenCalled()
	})
})
