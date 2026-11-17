export interface DemoApproval {
	title: string
	decision: string
	date: string
	approved: boolean
}

export const DEMO_APPROVAL_HISTORY: ReadonlyArray<DemoApproval> = [
	{
		title: "Embauche de l'agent « Anouk (Design) »",
		decision: 'Approuvée par François-Guillaume Ribreau',
		date: '12 nov.',
		approved: true,
	},
	{
		title: 'Dépassement de budget de Dev (cycle autonome) : +40 $',
		decision: 'Refusée par François-Guillaume Ribreau',
		date: '3 nov.',
		approved: false,
	},
	{
		title: 'Accès en écriture au Drive « compta/ »',
		decision: 'Approuvée par Baptiste Parmantier',
		date: '29 oct.',
		approved: true,
	},
]
