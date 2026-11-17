import { fireEvent, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { demoStore, resetDemoState } from '#/domain/demo'
import { SourceNewPage } from '#/domain/sources/feature/source-new-page'

const EXTRA = ['/sources/nouvelle', '/sources/$sourceId/{-$tab}']

describe('SourceNewPage', () => {
	beforeEach(resetDemoState)

	it('renders title, banner and six type tiles', async () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		await renderAtPath(<SourceNewPage />, '/sources/nouvelle', EXTRA)
		expect(
			await screen.findByRole('heading', { name: 'Nouvelle source' }),
		).toBeTruthy()
		expect(screen.getByRole('status').textContent).toContain('démonstration')
		expect(screen.getAllByRole('button', { pressed: false })).toHaveLength(5)
		expect(screen.getByText(/Exemple :/).textContent).toContain(
			'iam.exemple.fr',
		)
		expect(fetchSpy).not.toHaveBeenCalled()
	})

	it('switching type resets defaults and keeps the name', async () => {
		await renderAtPath(<SourceNewPage />, '/sources/nouvelle', EXTRA)
		fireEvent.change(await screen.findByLabelText('Nom'), {
			target: { value: 'Ma boîte' },
		})
		fireEvent.click(screen.getByRole('button', { name: /Boîte e-mail/ }))
		expect((screen.getByLabelText('Nom') as HTMLInputElement).value).toBe(
			'Ma boîte',
		)
		expect(
			(screen.getByLabelText('Adresses ou expéditeurs') as HTMLInputElement)
				.value,
		).toBe('cabinet@natalia.fr')
		expect(
			(screen.getByLabelText('Exclusions') as HTMLInputElement).value,
		).toBe('')
		expect(
			(screen.getByLabelText('Titre de la tâche') as HTMLInputElement).value,
		).toBe('Mail reçu : {valeur}')
	})

	it('refuses an incomplete form', async () => {
		await renderAtPath(<SourceNewPage />, '/sources/nouvelle', EXTRA)
		fireEvent.click(
			await screen.findByRole('button', { name: 'Créer la source' }),
		)
		expect((await screen.findByRole('alert')).textContent).toBe(
			'Renseigne un nom et au moins un motif',
		)
		expect(demoStore.state.sources).toHaveLength(3)
	})

	it('creates the source and navigates to it', async () => {
		await renderAtPath(<SourceNewPage />, '/sources/nouvelle', EXTRA)
		fireEvent.change(await screen.findByLabelText('Nom'), {
			target: { value: 'Mes sous-domaines' },
		})
		fireEvent.click(screen.getByRole('button', { name: 'Créer la source' }))
		await vi.waitFor(() => expect(demoStore.state.sources).toHaveLength(4))
		const created = demoStore.state.sources[3]
		expect(created).toMatchObject({
			id: 'mes-sous-domaines',
			patterns: ['iam.*', 'auth.*'],
			agentIds: ['veille-legale'],
		})
	})
})
