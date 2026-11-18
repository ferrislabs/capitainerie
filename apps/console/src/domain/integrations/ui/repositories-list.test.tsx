import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	filterRepositories,
	RepositoriesList,
} from '#/domain/integrations/ui/repositories-list'

const WEB: Schemas.RepositoryResponse = {
	owner: 'acme',
	name: 'web',
	full_name: 'acme/web',
	private: true,
	html_url: 'https://github.com/acme/web',
}
const API: Schemas.RepositoryResponse = {
	owner: 'acme',
	name: 'api',
	full_name: 'acme/api',
	private: false,
	html_url: 'https://github.com/acme/api',
}

describe('RepositoriesList', () => {
	it('lists every repository with its visibility and a link to GitHub', () => {
		render(<RepositoriesList repositories={[WEB, API]} />)

		const rows = screen.getAllByRole('listitem')
		expect(rows).toHaveLength(2)
		expect(within(rows[0]).getByText('acme/web')).toBeTruthy()
		expect(within(rows[0]).getByText('Privé')).toBeTruthy()
		expect(within(rows[1]).getByText('Public')).toBeTruthy()
		expect(
			within(rows[0])
				.getByRole('link', { name: 'Ouvrir acme/web sur GitHub' })
				.getAttribute('href'),
		).toBe('https://github.com/acme/web')
		expect(screen.getByText('2 dépôts accessibles')).toBeTruthy()
	})

	it('narrows the list as the user types and says when nothing matches', async () => {
		render(<RepositoriesList repositories={[WEB, API]} />)
		const search = screen.getByRole('searchbox', {
			name: 'Rechercher un dépôt',
		})

		await userEvent.type(search, 'API')
		expect(screen.getAllByRole('listitem')).toHaveLength(1)
		expect(screen.getByText('acme/api')).toBeTruthy()

		await userEvent.clear(search)
		await userEvent.type(search, 'zzz')
		expect(screen.queryAllByRole('listitem')).toHaveLength(0)
		expect(screen.getByText(/Aucun dépôt ne correspond à « zzz »/)).toBeTruthy()
	})

	it('explains an installation that reaches no repository', () => {
		render(<RepositoriesList repositories={[]} />)

		expect(screen.getByText(/n'a accès à aucun dépôt/)).toBeTruthy()
		expect(screen.queryByRole('searchbox')).toBeNull()
	})

	it('filters by a case-insensitive part of the full name', () => {
		expect(filterRepositories([WEB, API], ' WE ')).toEqual([WEB])
		expect(filterRepositories([WEB, API], '')).toEqual([WEB, API])
	})
})
