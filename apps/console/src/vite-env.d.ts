/// <reference types="vite/client" />

interface ImportMetaEnv {
	readonly VITE_API_URL?: string
	readonly VITE_OIDC_ISSUER_URL?: string
	readonly VITE_OIDC_CLIENT_ID?: string
	readonly VITE_OIDC_RESOURCE?: string
}

interface ImportMeta {
	readonly env: ImportMetaEnv
}
