import type { UserManager } from 'oidc-client-ts'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
	adoptRenewalsFromOtherTabs,
	createUserManager,
	getUserManager,
	leadRenewalsForThisTab,
	RENEWAL_LOCK,
	userStorageKey,
} from '#/lib/oidc'

const AUTHORITY = 'http://idp.test/realms/capitainerie'
const CLIENT_ID = 'capitainerie-console'

vi.mock('#/lib/runtime-config', () => ({
	getOidcConfiguration: () => ({
		authority: AUTHORITY,
		client_id: CLIENT_ID,
		redirect_uri: 'http://app.test/',
		scope: 'openid profile email',
	}),
}))

interface RenewalStub {
	startSilentRenew: ReturnType<typeof vi.fn>
	getUser: ReturnType<typeof vi.fn>
	events: { unload: ReturnType<typeof vi.fn> }
	settings: { authority: string; client_id: string }
}

function renewalStub(): RenewalStub {
	return {
		startSilentRenew: vi.fn(),
		getUser: vi.fn().mockResolvedValue(null),
		events: { unload: vi.fn().mockResolvedValue(undefined) },
		settings: { authority: AUTHORITY, client_id: CLIENT_ID },
	}
}

function asUserManager(stub: RenewalStub): UserManager {
	return stub as unknown as UserManager
}

describe('userStorageKey', () => {
	it('matches the key oidc-client-ts actually writes', async () => {
		const manager = createUserManager()
		if (!manager) throw new Error('user manager expected')

		window.localStorage.clear()
		await manager.storeUser({ toStorageString: () => '{}' } as never)

		expect(window.localStorage.key(0)).toBe(
			userStorageKey(AUTHORITY, CLIENT_ID),
		)
	})
})

describe('createUserManager', () => {
	it('never renews on its own', () => {
		expect(createUserManager()?.settings.automaticSilentRenew).toBe(false)
	})
})

describe('getUserManager', () => {
	it('returns a single instance per document', () => {
		expect(getUserManager()).toBe(getUserManager())
	})
})

describe('leadRenewalsForThisTab', () => {
	beforeEach(() => {
		vi.unstubAllGlobals()
	})

	it('renews only once the lock is held, under the shared lock name', () => {
		const stub = renewalStub()
		let release: (() => void) | undefined
		const request = vi.fn(
			(_name: string, callback: () => Promise<never>) =>
				new Promise<void>((resolve) => {
					release = () => {
						void callback()
						resolve()
					}
				}),
		)
		vi.stubGlobal('navigator', { locks: { request } })

		void leadRenewalsForThisTab(asUserManager(stub))
		expect(request).toHaveBeenCalledWith(RENEWAL_LOCK, expect.any(Function))
		expect(stub.startSilentRenew).not.toHaveBeenCalled()

		release?.()
		expect(stub.startSilentRenew).toHaveBeenCalledOnce()
	})

	it('keeps holding the lock', async () => {
		const stub = renewalStub()
		let held: Promise<unknown> | undefined
		vi.stubGlobal('navigator', {
			locks: {
				request: (_name: string, callback: () => Promise<never>) => {
					held = callback()
					return new Promise<void>(() => {})
				},
			},
		})

		void leadRenewalsForThisTab(asUserManager(stub))

		const settled = await Promise.race([
			held?.then(() => 'settled'),
			Promise.resolve('pending'),
		])
		expect(settled).toBe('pending')
	})

	it('renews directly when Web Locks are unavailable', async () => {
		const stub = renewalStub()
		vi.stubGlobal('navigator', {})

		await leadRenewalsForThisTab(asUserManager(stub))

		expect(stub.startSilentRenew).toHaveBeenCalledOnce()
	})
})

describe('adoptRenewalsFromOtherTabs', () => {
	it('adopts the token written by the leading tab', () => {
		const stub = renewalStub()
		adoptRenewalsFromOtherTabs(asUserManager(stub))

		window.dispatchEvent(
			new StorageEvent('storage', {
				key: userStorageKey(AUTHORITY, CLIENT_ID),
				newValue: '{"access_token":"x"}',
			}),
		)

		expect(stub.getUser).toHaveBeenCalledWith(true)
	})

	it('unloads when the session is cleared elsewhere', () => {
		const stub = renewalStub()
		adoptRenewalsFromOtherTabs(asUserManager(stub))

		window.dispatchEvent(
			new StorageEvent('storage', {
				key: userStorageKey(AUTHORITY, CLIENT_ID),
				newValue: null,
			}),
		)

		expect(stub.events.unload).toHaveBeenCalledOnce()
		expect(stub.getUser).not.toHaveBeenCalled()
	})

	it('ignores unrelated keys and stops once unsubscribed', () => {
		const stub = renewalStub()
		const stop = adoptRenewalsFromOtherTabs(asUserManager(stub))

		window.dispatchEvent(
			new StorageEvent('storage', { key: 'other', newValue: 'x' }),
		)
		stop()
		window.dispatchEvent(
			new StorageEvent('storage', {
				key: userStorageKey(AUTHORITY, CLIENT_ID),
				newValue: 'x',
			}),
		)

		expect(stub.getUser).not.toHaveBeenCalled()
		expect(stub.events.unload).not.toHaveBeenCalled()
	})
})
