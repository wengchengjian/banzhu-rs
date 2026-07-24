import { ref } from 'vue'

interface ConfirmOptions {
  title?: string
  message: string
  confirmText?: string
  cancelText?: string
}

const visible = ref(false)
const options = ref<ConfirmOptions>({ message: '' })
let resolver: ((value: boolean) => void) | null = null

export function useConfirm() {
  function confirm(opts: ConfirmOptions): Promise<boolean> {
    options.value = opts
    visible.value = true
    return new Promise<boolean>(resolve => { resolver = resolve })
  }
  function resolve(value: boolean) {
    visible.value = false
    resolver?.(value)
    resolver = null
  }
  return { visible, options, confirm, resolve }
}
