import { TabNav } from '#/components/layout/tab-nav'

export function AgentsTabs() {
	return (
		<TabNav
			label="Sections des agents"
			tabs={[
				{
					label: 'Équipe',
					link: { to: '/agents', activeOptions: { exact: true } },
				},
				{ label: 'Organigramme', link: { to: '/agents/organigramme' } },
			]}
		/>
	)
}
