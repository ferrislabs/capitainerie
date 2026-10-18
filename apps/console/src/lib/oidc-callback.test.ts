import { describe, expect, it } from 'vitest'
import { isSigninCallback } from '#/lib/oidc'

function at(url: string): Location {
	return new URL(url) as unknown as Location
}

describe('isSigninCallback', () => {
	const redirectUri = 'http://localhost:4080/callback'

	it('treats the OIDC redirect page as a sign-in response', () => {
		expect(
			isSigninCallback(
				at('http://localhost:4080/callback?code=a&state=b'),
				redirectUri,
			),
		).toBe(true)
	})

	it('leaves the GitHub callbacks to their own pages', () => {
		for (const path of [
			'/settings/integrations/github/app-callback?code=a&state=b',
			'/settings/integrations/github/callback?code=a&state=b&installation_id=1',
		]) {
			expect(
				isSigninCallback(at(`http://localhost:4080${path}`), redirectUri),
			).toBe(false)
		}
	})

	it('accepts a relative redirect URI', () => {
		expect(
			isSigninCallback(at('http://localhost:4080/callback'), '/callback'),
		).toBe(true)
	})
})
