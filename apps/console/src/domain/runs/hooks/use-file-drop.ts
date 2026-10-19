import { type DragEvent, useState } from 'react'

export function useFileDrop(onFile: (file: File) => void, disabled: boolean) {
	const [isDragging, setIsDragging] = useState(false)

	return {
		isDragging,
		handlers: {
			onDragEnter(event: DragEvent<HTMLElement>) {
				event.preventDefault()
				if (!disabled) setIsDragging(true)
			},
			onDragOver(event: DragEvent<HTMLElement>) {
				event.preventDefault()
			},
			onDragLeave(event: DragEvent<HTMLElement>) {
				const next = event.relatedTarget
				if (!(next instanceof Node) || !event.currentTarget.contains(next)) {
					setIsDragging(false)
				}
			},
			onDrop(event: DragEvent<HTMLElement>) {
				event.preventDefault()
				setIsDragging(false)
				const file = event.dataTransfer.files[0]
				if (file && !disabled) onFile(file)
			},
		},
	}
}
