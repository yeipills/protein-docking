type ToastType = 'success' | 'error' | 'info' | 'warning'

interface ToastOptions {
  message: string
  type: ToastType
  duration?: number
}

class ToastManager {
  private container: HTMLElement | null = null

  private getContainer(): HTMLElement {
    if (!this.container) {
      this.container = document.createElement('div')
      this.container.id = 'toast-container'
      this.container.className =
        'fixed top-4 right-4 z-50 flex flex-col gap-2 pointer-events-none'
      document.body.appendChild(this.container)
    }
    return this.container
  }

  private show(options: ToastOptions) {
    const { message, type, duration = 3000 } = options
    const container = this.getContainer()

    const toast = document.createElement('div')
    toast.className = `
      pointer-events-auto
      px-4 py-3 rounded-lg shadow-lg
      flex items-center gap-3
      animate-slide-in
      ${this.getTypeClasses(type)}
    `

    const icon = this.getIcon(type)
    toast.innerHTML = `
      ${icon}
      <span class="text-sm font-medium">${message}</span>
    `

    container.appendChild(toast)

    setTimeout(() => {
      toast.style.animation = 'fadeOut 0.3s ease-out'
      setTimeout(() => {
        container.removeChild(toast)
      }, 300)
    }, duration)
  }

  private getTypeClasses(type: ToastType): string {
    switch (type) {
      case 'success':
        return 'bg-green-500 text-white'
      case 'error':
        return 'bg-red-500 text-white'
      case 'warning':
        return 'bg-yellow-500 text-white'
      case 'info':
        return 'bg-blue-500 text-white'
    }
  }

  private getIcon(type: ToastType): string {
    switch (type) {
      case 'success':
        return '<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path></svg>'
      case 'error':
        return '<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>'
      case 'warning':
        return '<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path></svg>'
      case 'info':
        return '<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>'
    }
  }

  success(message: string, duration?: number) {
    this.show({ message, type: 'success', duration })
  }

  error(message: string, duration?: number) {
    this.show({ message, type: 'error', duration })
  }

  warning(message: string, duration?: number) {
    this.show({ message, type: 'warning', duration })
  }

  info(message: string, duration?: number) {
    this.show({ message, type: 'info', duration })
  }
}

export const toast = new ToastManager()
