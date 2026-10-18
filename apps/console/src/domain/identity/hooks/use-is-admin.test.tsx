import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { useIsAdmin } from '#/domain/identity/hooks/use-is-admin'
import { ADMIN_ROLE, realmRoles } from '#/domain/identity/roles'
import { clearAuth, setAuth } from '#/store/auth.store'

function base64Url(value: string): string {
	const bytes = new TextEncoder().encode(value)
	const binary = String.fromCharCode(...bytes)
	return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

function token(claims: unknown): string {
	return `${base64Url('{"alg":"RS256"}')}.${base64Url(JSON.stringify(claims))}.sig`
}

describe('realmRoles', () => {
	it('reads realm_access.roles, including non-ASCII claims', () => {
		expect(
			realmRoles(
				token({ name: 'Éloïse', realm_access: { roles: ['a', ADMIN_ROLE] } }),
			),
		).toEqual(['a', ADMIN_ROLE])
	})

	it('returns no role for missing or malformed tokens', () => {
		expect(realmRoles(null)).toEqual([])
		expect(realmRoles('not-a-jwt')).toEqual([])
		expect(realmRoles('a.%%%.c')).toEqual([])
		expect(realmRoles(token({ realm_access: { roles: 'admin' } }))).toEqual([])
	})
})

describe('useIsAdmin', () => {
	afterEach(() => clearAuth())

	it('is false without a token', () => {
		const { result } = renderHook(() => useIsAdmin())
		expect(result.current).toBe(false)
	})

	it('is false for a member token', () => {
		setAuth({ accessToken: token({ realm_access: { roles: ['member'] } }) })
		const { result } = renderHook(() => useIsAdmin())
		expect(result.current).toBe(false)
	})

	it('follows the token in the auth store', () => {
		const { result } = renderHook(() => useIsAdmin())

		act(() =>
			setAuth({
				accessToken: token({ realm_access: { roles: [ADMIN_ROLE] } }),
			}),
		)
		expect(result.current).toBe(true)

		act(() => clearAuth())
		expect(result.current).toBe(false)
	})
})
