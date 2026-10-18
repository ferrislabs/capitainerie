import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { ModelsPage } from '#/domain/settings/feature/models-page'
import { ProvidersPage } from '#/domain/settings/feature/providers-page'

describe('settings placeholders', () => {
	it('renders the providers page as coming soon', () => {
		render(<ProvidersPage />)

		expect(
			screen.getByRole('heading', { level: 1, name: 'Fournisseurs' }),
		).toBeTruthy()
		expect(screen.getByText('Bientôt disponible')).toBeTruthy()
		expect(screen.getByText(/Anthropic, Mistral/)).toBeTruthy()
	})

	it('renders the models page as coming soon', () => {
		render(<ModelsPage />)

		expect(
			screen.getByRole('heading', { level: 1, name: 'Modèles' }),
		).toBeTruthy()
		expect(screen.getByText('Bientôt disponible')).toBeTruthy()
		expect(
			screen.getByText(/extractor, judge, writer, embeddings/),
		).toBeTruthy()
	})
})
