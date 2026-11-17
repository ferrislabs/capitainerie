export const AUDIT_TABS = [
	{ slug: 'activite', label: 'Activité' },
	{ slug: 'runs', label: 'Runs' },
	{ slug: 'couts', label: 'Coûts' },
	{ slug: 'budgets', label: 'Budgets' },
	{ slug: 'chronologie', label: 'Chronologie' },
] as const

export type AuditTab = (typeof AUDIT_TABS)[number]['slug']

export function parseAuditTab(value: string | undefined): AuditTab {
	return AUDIT_TABS.find((tab) => tab.slug === value)?.slug ?? 'activite'
}
