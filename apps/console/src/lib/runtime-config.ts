export interface RuntimeConfig {
	apiUrl: string
	issuerUrl: string | undefined
	clientId: string | undefined
	resource: string | undefined
}

export interface OidcConfiguration {
	authority: string
	client_id: string
	redirect_uri: string
	scope: string
	resource: string | undefined
}

interface RawConfig {
	api_url?: string
	issuer_url?: string
	client_id?: string
	resource?: string
}

let config: RuntimeConfig = {
	apiUrl: '',
	issuerUrl: undefined,
	clientId: undefined,
	resource: undefined,
}
let loadingPromise: Promise<void> | null = null

export function isPlaceholder(value: string | undefined): boolean {
	if (!value) return true
	return value.startsWith('${') && value.endsWith('}')
}

function clean(value: string | undefined): string | undefined {
	return isPlaceholder(value) ? undefined : value
}

async function readRuntimeConfig(): Promise<RuntimeConfig> {
	if (import.meta.env.DEV) {
		return {
			apiUrl: import.meta.env.VITE_API_URL ?? '',
			issuerUrl: clean(import.meta.env.VITE_OIDC_ISSUER_URL),
			clientId: clean(import.meta.env.VITE_OIDC_CLIENT_ID),
			resource: clean(import.meta.env.VITE_OIDC_RESOURCE),
		}
	}

	try {
		const res = await fetch('/config.json', { cache: 'no-store' })
		if (res.ok) {
			const data: RawConfig = await res.json()
			return {
				apiUrl: clean(data.api_url) ?? '',
				issuerUrl: clean(data.issuer_url),
				clientId: clean(data.client_id),
				resource: clean(data.resource),
			}
		}
	} catch (err) {
		console.error('Failed to load /config.json', err)
	}
	return {
		apiUrl: '',
		issuerUrl: undefined,
		clientId: undefined,
		resource: undefined,
	}
}

export function loadRuntimeConfig(): Promise<void> {
	if (typeof window === 'undefined') return Promise.resolve()
	if (loadingPromise) return loadingPromise

	loadingPromise = readRuntimeConfig().then((loaded) => {
		config = loaded
	})
	return loadingPromise
}

export function getRuntimeConfig(): RuntimeConfig {
	return config
}

export function getOidcConfiguration(): OidcConfiguration | undefined {
	if (typeof window === 'undefined') return undefined
	if (!config.issuerUrl || !config.clientId) return undefined
	return {
		authority: config.issuerUrl,
		client_id: config.clientId,
		redirect_uri: `${window.location.origin}/callback`,
		scope: 'openid profile email',
		resource: config.resource,
	}
}
