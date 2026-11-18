import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from '#/components/ui/alert-dialog'

interface Props {
	open: boolean
	sourceName: string
	onOpenChange: (open: boolean) => void
	onConfirm: () => void
}

export function RotateTokenDialog({
	open,
	sourceName,
	onOpenChange,
	onConfirm,
}: Props) {
	return (
		<AlertDialog open={open} onOpenChange={onOpenChange}>
			<AlertDialogContent>
				<AlertDialogHeader>
					<AlertDialogTitle>Régénérer le token ?</AlertDialogTitle>
					<AlertDialogDescription>
						L'URL actuelle de « {sourceName} » cessera de fonctionner
						immédiatement. Il faudra mettre la nouvelle URL dans l'outil qui
						appelle ce webhook.
					</AlertDialogDescription>
				</AlertDialogHeader>
				<AlertDialogFooter>
					<AlertDialogCancel>Annuler</AlertDialogCancel>
					<AlertDialogAction onClick={onConfirm}>Régénérer</AlertDialogAction>
				</AlertDialogFooter>
			</AlertDialogContent>
		</AlertDialog>
	)
}
