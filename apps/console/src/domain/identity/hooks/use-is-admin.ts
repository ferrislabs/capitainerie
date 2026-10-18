import { useAccessToken } from '#/domain/identity/hooks/use-access-token'
import { isAdminToken } from '#/domain/identity/roles'

/** Indication pour l'interface seulement : l'API reste l'autorité (403). */
export function useIsAdmin(): boolean {
	return isAdminToken(useAccessToken())
}
