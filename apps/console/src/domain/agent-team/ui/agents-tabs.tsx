import { TabNav } from '#/components/layout/tab-nav'

export function AgentsTabs() {
	return (
		<TabNav
			label="Sections des agents"
			tabs={[
				{
					label: 'Activité et coûts',
					link: { to: '/agents', activeOptions: { exact: true } },
				},
				{ label: 'Équipe', link: { to: '/agents/equipe' } },
				{ label: 'Organigramme', link: { to: '/agents/organigramme' } },
			]}
		/>
	)
}
