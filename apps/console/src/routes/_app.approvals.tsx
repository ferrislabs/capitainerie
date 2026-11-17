import { createFileRoute } from '@tanstack/react-router'
import { ApprovalsPage } from '#/domain/approvals/feature/approvals-page'

export const Route = createFileRoute('/_app/approvals')({
	component: ApprovalsPage,
})
