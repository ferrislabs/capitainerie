import { Button } from '#/components/ui/button'

interface Props {
	onCancel: () => void
	onSave: () => void
}

export function SaveBar({ onCancel, onSave }: Props) {
	return (
		<div className="sticky bottom-3 z-10 flex items-center gap-2 rounded-lg border bg-card px-3 py-2 shadow-md">
			<span className="flex-1 font-medium">Modifications non enregistrées</span>
			<Button type="button" variant="outline" onClick={onCancel}>
				Annuler
			</Button>
			<Button type="button" onClick={onSave}>
				Enregistrer
			</Button>
		</div>
	)
}
