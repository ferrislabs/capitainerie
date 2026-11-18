import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { renderAtPath } from '#/components/layout/test-router'
import {
	countWorksites,
	type IssuesState,
	LaunchWorksitesDialog,
	MAX_WORKSITES,
	type RepositoriesState,
} from '#/domain/worksites/ui/launch-worksites-dialog'

const READY: RepositoriesState = {
	kind: 'ready',
	repositories: [
		{
			full_name: 'acme/web',
			name: 'web',
			owner: 'acme',
			private: false,
			html_url: 'https://github.com/acme/web',
		},
	],
}

const ISSUES: ReadonlyArray<Schemas.IssueResponse> = [
	{ number: 7, title: 'Crash on empty input', html_url: 'u7', labels: [] },
	{
		number: 9,
		title: 'Dark mode',
		html_url: 'u9',
		labels: [],
		card_id: 'card-9',
	},
]

function setup(
	overrides: Partial<Parameters<typeof LaunchWorksitesDialog>[0]> = {},
) {
	const handlers = {
		onRepositoryChange: vi.fn(),
		onPromptChange: vi.fn(),
		onAddPrompt: vi.fn(),
		onRemovePrompt: vi.fn(),
		onToggleIssue: vi.fn(),
		onRetry: vi.fn(),
		onSubmit: vi.fn(),
		onClose: vi.fn(),
	}
	render(
		<LaunchWorksitesDialog
			open
			projectName="Site vitrine"
			repositories={READY}
			repository="acme/web"
			prompts={['']}
			issues={{ kind: 'ready', issues: ISSUES }}
			selectedIssues={[]}
			isPending={false}
			{...handlers}
			{...overrides}
		/>,
	)
	return handlers
}

describe('countWorksites', () => {
	it('counts filled prompts and selected issues, not blank prompts', () => {
		expect(countWorksites(['a', '  ', ''], [7, 8])).toBe(3)
		expect(countWorksites([''], [])).toBe(0)
	})
})

describe('LaunchWorksitesDialog', () => {
	it('names the project and keeps the launch disabled while nothing is filled in', () => {
		setup()

		expect(screen.getByText(/projet « Site vitrine »/)).toBeTruthy()
		expect(
			screen.getByRole('button', { name: 'Lancer le chantier' }),
		).toHaveProperty('disabled', true)
	})

	it('launches one worksite per filled prompt and selected issue', async () => {
		const handlers = setup({
			prompts: ['Ajouter un CHANGELOG', 'Corriger le footer'],
			selectedIssues: [7],
		})

		const launch = screen.getByRole('button', { name: 'Lancer 3 chantiers' })
		await userEvent.click(launch)

		expect(handlers.onSubmit).toHaveBeenCalledOnce()
	})

	it('reports edits of a prompt and lets the user add or remove one', async () => {
		const handlers = setup({ prompts: ['a', 'b'] })

		await userEvent.type(screen.getByLabelText('Prompt 1'), 'x')
		await userEvent.click(
			screen.getByRole('button', { name: 'Ajouter un prompt' }),
		)
		await userEvent.click(
			screen.getByRole('button', { name: 'Retirer le prompt 2' }),
		)

		expect(handlers.onPromptChange).toHaveBeenCalledWith(0, 'ax')
		expect(handlers.onAddPrompt).toHaveBeenCalledOnce()
		expect(handlers.onRemovePrompt).toHaveBeenCalledWith(1)
	})

	it('cannot remove the only prompt', () => {
		setup({ prompts: [''] })

		expect(
			screen.queryByRole('button', { name: /Retirer le prompt/ }),
		).toBeNull()
	})

	it('selects an open issue, and refuses one that already has a worksite', async () => {
		const handlers = setup()

		await userEvent.click(
			screen.getByRole('checkbox', { name: /Crash on empty input/ }),
		)

		expect(handlers.onToggleIssue).toHaveBeenCalledWith(7, true)
		const taken = screen.getByRole('checkbox', { name: /Dark mode/ })
		expect(taken).toHaveProperty('disabled', true)
		expect(screen.getByText('déjà un chantier')).toBeTruthy()
	})

	it.each<[IssuesState, RegExp]>([
		[{ kind: 'idle' }, /Choisis un dépôt/],
		[
			{ kind: 'error', message: 'Permission manquante' },
			/Permission manquante/,
		],
		[{ kind: 'ready', issues: [] }, /aucune issue ouverte/],
	])('says what is going on with the issues (%o)', (issues, expected) => {
		setup({ issues })

		expect(screen.getByText(expected)).toBeTruthy()
	})

	it('refuses more worksites than the server accepts', () => {
		const prompts = Array.from({ length: MAX_WORKSITES + 1 }, (_, n) => `p${n}`)
		setup({ prompts })

		expect(
			screen.getByText(
				`Au plus ${MAX_WORKSITES} chantiers à la fois : il y en a ${MAX_WORKSITES + 1}.`,
			),
		).toBeTruthy()
		expect(
			screen.getByRole('button', {
				name: `Lancer ${MAX_WORKSITES + 1} chantiers`,
			}),
		).toHaveProperty('disabled', true)
	})

	it('blocks the form while the launch is pending', () => {
		setup({ prompts: ['a'], isPending: true })

		expect(screen.getByRole('button', { name: 'Envoi…' })).toHaveProperty(
			'disabled',
			true,
		)
		expect(screen.getByRole('button', { name: 'Annuler' })).toHaveProperty(
			'disabled',
			true,
		)
	})

	it('shows the server error', () => {
		setup({ errorMessage: 'Une des issues a déjà un chantier.' })

		expect(screen.getByText('Une des issues a déjà un chantier.')).toBeTruthy()
	})

	it('says GitHub is not connected, links to its settings and offers no launch', async () => {
		await renderAtPath(
			<LaunchWorksitesDialog
				open
				projectName="Site vitrine"
				repositories={{ kind: 'not_connected' }}
				repository=""
				prompts={['']}
				issues={{ kind: 'ready', issues: [] }}
				selectedIssues={[]}
				isPending={false}
				onRepositoryChange={vi.fn()}
				onPromptChange={vi.fn()}
				onAddPrompt={vi.fn()}
				onRemovePrompt={vi.fn()}
				onToggleIssue={vi.fn()}
				onRetry={vi.fn()}
				onSubmit={vi.fn()}
				onClose={vi.fn()}
			/>,
			'/worksites',
		)

		expect(screen.getByText(/GitHub n'est pas connecté/)).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: 'Connecter GitHub' })
				.getAttribute('href'),
		).toBe('/settings/integrations/github')
		expect(screen.queryByRole('button', { name: /Lancer/ })).toBeNull()
	})
})
