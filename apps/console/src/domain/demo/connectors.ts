/** Autorisations de départ : refus par défaut, seuls les accès listés existent. */
export const DEMO_GRANTS: Readonly<Record<string, ReadonlyArray<string>>> = {
	'security-officer': ['drive-steward (MCP)'],
	archibald: ['Chatwoot', 'Archivage Inexweb vers Drive'],
	agathe: ['Chatwoot', 'Clients facturés (lecture seule)'],
	dev: ['GitLab (merge requests)'],
	ferdinand: ['GitLab (merge requests)'],
	'veille-legale': [
		'RDAP (domaines)',
		'RNE · Annuaire des entreprises',
		'BODACC',
		'Odoo CRM (MCP)',
	],
	registre: ['RNE · Annuaire des entreprises', 'BODACC'],
	'rdap-dns': ['RDAP (domaines)'],
}
