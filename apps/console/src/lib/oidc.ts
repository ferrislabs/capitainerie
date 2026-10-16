import { UserManager, WebStorageStateStore } from 'oidc-client-ts'
import { getOidcConfiguration } from '#/lib/runtime-config'

export const RENEWAL_LOCK = 'capitainerie.oidc.silent-renew'

export function userStorageKey(authority: string, clientId: string): string {
	return `oidc.user:${authority}:${clientId}`
}

export function createUserManager(): UserManager | null {
	if (typeof window === 'undefined') return null
	const cfg = getOidcConfiguration()
	if (!cfg) return null

	return new UserManager({
		authority: cfg.authority,
		client_id: cfg.client_id,
		redirect_uri: cfg.redirect_uri,
		scope: cfg.scope,
		post_logout_redirect_uri: `${window.location.origin}/`,
		response_type: 'code',
		loadUserInfo: true,
		userStore: new WebStorageStateStore({ store: window.localStorage }),
		automaticSilentRenew: false,
		extraQueryParams: cfg.resource ? { resource: cfg.resource } : undefined,
	})
}

export function leadRenewalsForThisTab(
	userManager: UserManager,
): Promise<void> {
	if (typeof navigator === 'undefined' || !navigator.locks) {
		userManager.startSilentRenew()
		return Promise.resolve()
	}

	return navigator.locks.request<void>(RENEWAL_LOCK, async () => {
		userManager.startSilentRenew()
		await new Promise<never>(() => {})
	})
}

export function adoptRenewalsFromOtherTabs(
	userManager: UserManager,
): () => void {
	const key = userStorageKey(
		userManager.settings.authority,
		userManager.settings.client_id,
	)

	const onStorage = (event: StorageEvent) => {
		if (event.key !== key) return

		if (event.newValue === null) {
			void userManager.events.unload()
			return
		}

		void userManager.getUser(true)
	}

	window.addEventListener('storage', onStorage)
	return () => window.removeEventListener('storage', onStorage)
}

let instance: UserManager | null | undefined

export function getUserManager(): UserManager | null {
	if (instance !== undefined) return instance

	instance = createUserManager()
	if (instance) {
		adoptRenewalsFromOtherTabs(instance)
		void leadRenewalsForThisTab(instance)
	}

	return instance
}
