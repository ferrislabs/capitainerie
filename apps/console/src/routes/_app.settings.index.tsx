import { createFileRoute } from '@tanstack/react-router'
import { SettingsIndexPage } from '#/domain/settings/feature/settings-index-page'

export const Route = createFileRoute('/_app/settings/')({
	component: SettingsIndexPage,
})
