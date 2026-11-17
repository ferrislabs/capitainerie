import type { DemoConnector } from '#/domain/demo/types'

export const DEMO_CONNECTORS: ReadonlyArray<DemoConnector> = [
	{
		name: 'Chatwoot',
		kind: 'Canal',
		description: 'Conversations du support : réponses et notes privées.',
		accounts: ['Support Natalia'],
		tools: [
			{
				name: 'chatwoot_reply',
				capability: 'Répondre au contact',
				risk: 'envoi externe',
			},
			{
				name: 'chatwoot_note',
				capability: 'Note privée',
				risk: 'écriture limitée',
			},
		],
	},
	{
		name: 'E-mail',
		kind: 'Canal',
		description: "Boîte partagée de l'agent.",
		accounts: [],
		tools: [
			{ name: 'mail_read', capability: 'Lire la boîte', risk: 'lecture' },
			{
				name: 'mail_send',
				capability: 'Envoyer un e-mail',
				risk: 'envoi externe',
			},
		],
	},
	{
		name: 'Slack',
		kind: 'Canal',
		description: "Messages directs et canal d'équipe.",
		accounts: [],
		tools: [
			{
				name: 'slack_post',
				capability: 'Publier dans un canal',
				risk: 'envoi externe',
			},
		],
	},
	{
		name: 'WhatsApp',
		kind: 'Canal',
		description: 'Numéro Natalia.',
		accounts: [],
		tools: [
			{
				name: 'wa_send',
				capability: 'Envoyer un message',
				risk: 'envoi externe',
			},
		],
	},
	{
		name: 'drive-steward (MCP)',
		kind: 'MCP',
		description: 'Audit des partages, inventaire et journal du Drive.',
		accounts: ['drive-steward-natalia-secu'],
		tools: [
			{
				name: 'audit-sharing',
				capability: 'Audit des partages',
				risk: 'lecture',
			},
			{
				name: 'inventory',
				capability: 'Inventaire du périmètre',
				risk: 'lecture',
			},
			{ name: 'journal', capability: 'Relire le journal', risk: 'lecture' },
		],
	},
	{
		name: 'Odoo CRM (MCP)',
		kind: 'MCP',
		description: 'Lecture et écriture des fiches CRM.',
		accounts: ['odoo-natalia'],
		tools: [
			{
				name: 'crm_lead_search',
				capability: 'Chercher un lead',
				risk: 'lecture',
			},
			{
				name: 'crm_lead_create',
				capability: 'Créer un lead',
				risk: 'écriture',
			},
			{
				name: 'crm_lead_update',
				capability: 'Mettre à jour un lead',
				risk: 'écriture',
			},
			{
				name: 'crm_note',
				capability: 'Ajouter une note',
				risk: 'écriture limitée',
			},
		],
	},
	{
		name: 'n8n',
		kind: 'MCP',
		description: 'Déclenchement de workflows.',
		accounts: [],
		tools: [
			{
				name: 'workflow_execute',
				capability: 'Lancer un workflow',
				risk: 'écriture',
			},
		],
	},
	{
		name: 'Pennylane',
		kind: 'Outil',
		description: 'Dépôt de pièces comptables.',
		accounts: [],
		tools: [
			{
				name: 'depot_piece',
				capability: 'Déposer une pièce',
				risk: 'écriture',
			},
		],
	},
	{
		name: 'Archivage Inexweb vers Drive',
		kind: 'Outil',
		description:
			"Liste le portail Inexweb et archive un document sous compta/ du Drive. Les octets du PDF ne passent jamais par l'agent.",
		accounts: ['Inexweb Archive Natalia'],
		tools: [
			{
				name: 'inexweb_list',
				capability: 'Lister le portail',
				risk: 'lecture',
			},
			{
				name: 'drive_archive',
				capability: 'Archiver sous compta/',
				risk: 'écriture',
			},
		],
	},
	{
		name: 'Clients facturés (lecture seule)',
		kind: 'Outil',
		description:
			'Liste bornée des clients facturés sur 12 mois ; refuse toute clé en écriture.',
		accounts: ['Facturation lecture'],
		tools: [
			{
				name: 'billed_clients',
				capability: 'Clients facturés sur 12 mois',
				risk: 'lecture',
			},
		],
	},
	{
		name: 'GitLab (merge requests)',
		kind: 'Outil',
		description:
			'Création de MR, commentaires, relance des relecteurs. Jamais de merge sans relecture.',
		accounts: ['getnatalia/getnatalia'],
		tools: [
			{
				name: 'gitlab_mr_create',
				capability: 'Créer une MR',
				risk: 'écriture',
			},
			{ name: 'gitlab_mr_comment', capability: 'Commenter', risk: 'écriture' },
			{
				name: 'gitlab_reviewers',
				capability: 'Relancer les relecteurs',
				risk: 'écriture',
			},
		],
	},
	{
		name: 'RDAP (domaines)',
		kind: 'Registre',
		description:
			"Titulaire, registrar et dates d'un nom de domaine, historique DNS.",
		accounts: ['Accès public'],
		tools: [
			{
				name: 'rdap_domain',
				capability: 'Titulaire, registrar, dates',
				risk: 'lecture',
			},
			{ name: 'dns_history', capability: 'Historique DNS', risk: 'lecture' },
		],
	},
	{
		name: 'RNE · Annuaire des entreprises',
		kind: 'Registre',
		description:
			'Recherche par nom ou SIREN : fiche société, dirigeants, établissements.',
		accounts: ['Accès public'],
		tools: [
			{
				name: 'rne_search',
				capability: 'Recherche par nom ou SIREN',
				risk: 'lecture',
			},
			{
				name: 'rne_company',
				capability: 'Fiche société et dirigeants',
				risk: 'lecture',
			},
		],
	},
	{
		name: 'BODACC',
		kind: 'Registre',
		description: 'Annonces légales et procédures collectives.',
		accounts: ['Accès public'],
		tools: [
			{
				name: 'bodacc_search',
				capability: 'Annonces et procédures collectives',
				risk: 'lecture',
			},
		],
	},
]

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
