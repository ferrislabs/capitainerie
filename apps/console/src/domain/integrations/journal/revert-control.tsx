import { useId, useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { Button } from '#/components/ui/button'
import { RevertDialog } from '#/domain/integrations/journal/revert-dialog'
import { restoreFailure } from '#/domain/integrations/journal/revert-errors'
import { useRevertIntegrationAction } from '#/domain/integrations/journal/use-revert-action'

interface Props {
	action: Schemas.IntegrationActionResponse
	locked: boolean
	onRestored: () => void
}

export function RevertControl({ action, locked, onRestored }: Props) {
	const [open, setOpen] = useState(false)
	const revert = useRevertIntegrationAction()
	const reasonId = useId()

	function close() {
		revert.reset()
		setOpen(false)
	}

	function confirm(confirmProduction: boolean) {
		revert.mutate(
			{
				path: { id: action.id },
				body: confirmProduction ? { confirm_production: true } : {},
			},
			{
				onSuccess: (response) => {
					if (restoreFailure(response.items) !== undefined) return
					close()
					onRestored()
				},
			},
		)
	}

	return (
		<div className="flex flex-col items-start gap-1">
			<Button
				type="button"
				size="sm"
				variant="outline"
				disabled={locked}
				aria-describedby={locked ? reasonId : undefined}
				onClick={() => setOpen(true)}
			>
				Restaurer l’état d’avant
			</Button>
			{locked ? (
				<p id={reasonId} className="text-[11px] text-muted-foreground">
					L’intervention est encore en cours
				</p>
			) : null}
			{open ? (
				<RevertDialog
					action={action}
					isPending={revert.isPending}
					errorMessage={revert.errorMessage}
					onConfirm={confirm}
					onClose={close}
				/>
			) : null}
		</div>
	)
}
