import type { Dossier } from '#/domain/sources/criteria'

export const DOSSIERS: Readonly<Record<string, Dossier>> = {
	'iam.acme-logistique.fr': {
		raison: 'ACME LOGISTIQUE SAS',
		siren: '512 847 391',
		indices: 3,
		active: true,
		collective: false,
		effectif: 150,
		naf: '5229A',
		pays: 'FR',
		crm: false,
		rdapMasque: false,
		age: 2400,
		task: 'NAT-690',
	},
	'iam.novabank-groupe.eu': {
		raison: 'NOVABANK GROUPE',
		siren: '803 112 654',
		indices: 1,
		active: true,
		collective: false,
		effectif: null,
		naf: '6419Z',
		pays: 'FR',
		crm: false,
		rdapMasque: true,
		age: 900,
		task: 'NAT-689',
	},
	'auth.lumen-sante.com': {
		raison: 'LUMEN SANTE SARL',
		siren: '441 906 233',
		indices: 2,
		active: false,
		collective: true,
		effectif: 3,
		naf: '8690F',
		pays: 'FR',
		crm: true,
		age: 15,
		task: 'NAT-691',
	},
}

export const DOSSIER_SEGMENTS: ReadonlyArray<{ value: string; label: string }> =
	[
		{ value: 'iam.acme-logistique.fr', label: 'acme-logistique' },
		{ value: 'iam.novabank-groupe.eu', label: 'novabank' },
		{ value: 'auth.lumen-sante.com', label: 'lumen' },
	]
