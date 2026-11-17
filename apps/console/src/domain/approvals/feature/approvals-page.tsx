import { DEMO_APPROVAL_HISTORY } from '#/domain/approvals/mock/history'
import { ApprovalsView } from '#/domain/approvals/ui/approvals-view'

export function ApprovalsPage() {
	return <ApprovalsView history={DEMO_APPROVAL_HISTORY} />
}
