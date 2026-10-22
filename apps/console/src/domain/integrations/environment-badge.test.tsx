import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { EnvironmentBadge } from '#/domain/integrations/environment-badge'

describe('EnvironmentBadge', () => {
	it('labels each environment with its own tone', () => {
		const cases = [
			['production', 'Production', 'danger'],
			['staging', 'Staging', 'attention'],
			['development', 'Développement', 'neutral'],
		] as const
		for (const [environment, label, variant] of cases) {
			const { unmount } = render(<EnvironmentBadge environment={environment} />)
			expect(screen.getByText(label).getAttribute('data-variant')).toBe(variant)
			unmount()
		}
	})
})
