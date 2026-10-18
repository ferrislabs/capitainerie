import { render } from '@testing-library/react'
import { StrictMode } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ManifestForm } from '#/domain/integrations/feature/manifest-form'

describe('ManifestForm', () => {
	const submit = vi
		.spyOn(HTMLFormElement.prototype, 'submit')
		.mockImplementation(() => undefined)

	afterEach(() => submit.mockClear())

	it('posts the manifest to GitHub once, even under strict mode', () => {
		const manifest = '{"name":"Capitainerie Acme 1a2b3c"}'
		const { getByTestId } = render(
			<StrictMode>
				<ManifestForm
					actionUrl="https://github.com/settings/apps/new?state=s.t"
					manifest={manifest}
				/>
			</StrictMode>,
		)

		const form = getByTestId('github-manifest-form') as HTMLFormElement
		expect(form.method).toBe('post')
		expect(form.getAttribute('action')).toBe(
			'https://github.com/settings/apps/new?state=s.t',
		)
		expect(form.hidden).toBe(true)
		expect(
			(form.elements.namedItem('manifest') as HTMLInputElement).value,
		).toBe(manifest)
		expect(submit).toHaveBeenCalledTimes(1)
	})
})
