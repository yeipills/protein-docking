<!-- NotificationContainer - UI para mostrar notificaciones toast -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { slide, fade } from 'svelte/transition';
  import { notificationService, type Notification } from '../services/NotificationService';

  let notifications = $state<Notification[]>([]);
  let unsubscribe: (() => void) | null = null;

  onMount(() => {
    // Suscribirse a actualizaciones de notificaciones
    unsubscribe = notificationService.subscribe((newNotifications) => {
      notifications = newNotifications;
    });
  });

  onDestroy(() => {
    unsubscribe?.();
  });

  function dismiss(id: string) {
    notificationService.dismiss(id);
  }

  function dismissAll() {
    notificationService.dismissAll();
  }

  function executeAction(action: any) {
    try {
      action.action();
    } catch (error) {
      console.error('Error executing notification action:', error);
    }
  }

  // Formato de tiempo relativo
  function getRelativeTime(timestamp: number): string {
    const now = Date.now();
    const diff = now - timestamp;
    
    if (diff < 1000) return 'now';
    if (diff < 60000) return `${Math.floor(diff / 1000)}s ago`;
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
    return `${Math.floor(diff / 86400000)}d ago`;
  }

  // Obtener icono basado en tipo
  function getTypeIcon(type: string): string {
    switch (type) {
      case 'success': return '✅';
      case 'error': return '❌';
      case 'warning': return '⚠️';
      case 'info': return 'ℹ️';
      case 'loading': return '⏳';
      default: return '📢';
    }
  }

  // Obtener color basado en prioridad
  function getPriorityColor(priority: string): string {
    switch (priority) {
      case 'critical': return 'priority-critical';
      case 'high': return 'priority-high';
      case 'normal': return 'priority-normal';
      case 'low': return 'priority-low';
      default: return 'priority-normal';
    }
  }
</script>

<!-- Container de notificaciones -->
<div class="notification-container" role="alert" aria-live="polite">
  <!-- Header con contador y acciones -->
  {#if notifications.length > 0}
    <div class="notification-header">
      <span class="notification-count">
        {notifications.length} notification{notifications.length > 1 ? 's' : ''}
      </span>
      
      {#if notifications.length > 1}
        <button onclick={dismissAll} class="dismiss-all-btn" aria-label="Dismiss all notifications">
          <svg viewBox="0 0 20 20" fill="currentColor">
            <path d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z"/>
          </svg>
          Clear all
        </button>
      {/if}
    </div>
  {/if}

  <!-- Lista de notificaciones -->
  <div class="notification-list">
    {#each notifications as notification (notification.id)}
      <div 
        class="notification notification-{notification.type} {getPriorityColor(notification.priority)}"
        class:has-progress={notification.progress !== undefined}
        data-category={notification.category}
        in:slide={{ duration: 300, delay: 100 }}
        out:fade={{ duration: 200 }}
      >
        <!-- Barra de prioridad -->
        <div class="priority-bar"></div>
        
        <!-- Contenido principal -->
        <div class="notification-content">
          <!-- Header con icono y título -->
          <div class="notification-header-content">
            <div class="notification-icon">
              {getTypeIcon(notification.type)}
            </div>
            
            <div class="notification-text">
              <h4 class="notification-title">{notification.title}</h4>
              {#if notification.message}
                <p class="notification-message">{notification.message}</p>
              {/if}
            </div>
            
            <div class="notification-meta">
              <span class="notification-time">{getRelativeTime(notification.timestamp)}</span>
              <button 
                onclick={() => dismiss(notification.id)} 
                class="dismiss-btn"
                aria-label="Dismiss notification"
              >
                <svg viewBox="0 0 20 20" fill="currentColor">
                  <path d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z"/>
                </svg>
              </button>
            </div>
          </div>

          <!-- Barra de progreso si aplicable -->
          {#if notification.progress !== undefined}
            <div class="progress-container">
              <div class="progress-bar">
                <div 
                  class="progress-fill"
                  style="width: {notification.progress}%"
                ></div>
              </div>
              <span class="progress-text">{notification.progress.toFixed(1)}%</span>
            </div>
          {/if}

          <!-- Acciones disponibles -->
          {#if notification.actions && notification.actions.length > 0}
            <div class="notification-actions">
              {#each notification.actions as action}
                <button 
                  onclick={() => executeAction(action)}
                  class="action-btn action-{action.style || 'secondary'}"
                >
                  {action.label}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .notification-container {
    position: fixed;
    top: 1rem;
    right: 1rem;
    z-index: 9999;
    max-width: 400px;
    max-height: 80vh;
    overflow-y: auto;
    pointer-events: none;
  }

  .notification-container > * {
    pointer-events: all;
  }

  /* Header de notificaciones */
  .notification-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 1rem;
    background: rgba(0, 0, 0, 0.8);
    backdrop-filter: blur(10px);
    border-radius: 8px 8px 0 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    margin-bottom: 0.25rem;
  }

  .notification-count {
    font-size: 0.75rem;
    color: rgba(255, 255, 255, 0.8);
    font-weight: 500;
  }

  .dismiss-all-btn {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    background: rgba(255, 255, 255, 0.1);
    border: none;
    border-radius: 4px;
    color: rgba(255, 255, 255, 0.8);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .dismiss-all-btn:hover {
    background: rgba(255, 255, 255, 0.2);
    color: white;
  }

  .dismiss-all-btn svg {
    width: 12px;
    height: 12px;
  }

  /* Lista de notificaciones */
  .notification-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  /* Notificación individual */
  .notification {
    position: relative;
    background: white;
    border-radius: 8px;
    box-shadow: 
      0 4px 6px -1px rgba(0, 0, 0, 0.1),
      0 2px 4px -1px rgba(0, 0, 0, 0.06);
    border: 1px solid rgba(0, 0, 0, 0.1);
    overflow: hidden;
    animation: slideIn 0.3s ease-out;
    min-height: 60px;
  }

  @keyframes slideIn {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }

  /* Barra de prioridad lateral */
  .priority-bar {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 4px;
    background: var(--priority-color);
  }

  .priority-critical { --priority-color: #dc2626; }
  .priority-high { --priority-color: #f59e0b; }
  .priority-normal { --priority-color: #3b82f6; }
  .priority-low { --priority-color: #6b7280; }

  /* Contenido de notificación */
  .notification-content {
    padding: 1rem;
    padding-left: 1.25rem; /* Space for priority bar */
  }

  .notification-header-content {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .notification-icon {
    font-size: 1.25rem;
    line-height: 1;
    flex-shrink: 0;
    margin-top: 0.125rem;
  }

  .notification-text {
    flex: 1;
    min-width: 0;
  }

  .notification-title {
    margin: 0 0 0.25rem;
    font-size: 0.875rem;
    font-weight: 600;
    color: #111827;
    line-height: 1.2;
  }

  .notification-message {
    margin: 0;
    font-size: 0.8rem;
    color: #6b7280;
    line-height: 1.3;
    word-wrap: break-word;
  }

  .notification-meta {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.25rem;
    flex-shrink: 0;
  }

  .notification-time {
    font-size: 0.7rem;
    color: #9ca3af;
  }

  .dismiss-btn {
    padding: 0.25rem;
    background: none;
    border: none;
    color: #9ca3af;
    cursor: pointer;
    border-radius: 4px;
    transition: all 0.2s;
  }

  .dismiss-btn:hover {
    background: #f3f4f6;
    color: #6b7280;
  }

  .dismiss-btn svg {
    width: 16px;
    height: 16px;
  }

  /* Barra de progreso */
  .progress-container {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.75rem;
  }

  .progress-bar {
    flex: 1;
    height: 6px;
    background: #e5e7eb;
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #3b82f6, #1d4ed8);
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .progress-text {
    font-size: 0.7rem;
    color: #6b7280;
    font-weight: 500;
    min-width: 35px;
    text-align: right;
  }

  /* Acciones de notificación */
  .notification-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.75rem;
  }

  .action-btn {
    padding: 0.5rem 0.75rem;
    border: none;
    border-radius: 6px;
    font-size: 0.75rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .action-primary {
    background: #3b82f6;
    color: white;
  }

  .action-primary:hover {
    background: #2563eb;
  }

  .action-secondary {
    background: #f3f4f6;
    color: #374151;
    border: 1px solid #d1d5db;
  }

  .action-secondary:hover {
    background: #e5e7eb;
  }

  .action-danger {
    background: #ef4444;
    color: white;
  }

  .action-danger:hover {
    background: #dc2626;
  }

  /* Variantes por tipo */
  .notification-success {
    border-left: 4px solid #10b981;
  }

  .notification-error {
    border-left: 4px solid #ef4444;
  }

  .notification-warning {
    border-left: 4px solid #f59e0b;
  }

  .notification-info {
    border-left: 4px solid #3b82f6;
  }

  .notification-loading {
    border-left: 4px solid #8b5cf6;
  }

  .notification-loading .notification-icon {
    animation: pulse 2s infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  /* Responsive design */
  @media (max-width: 640px) {
    .notification-container {
      top: 0.5rem;
      right: 0.5rem;
      left: 0.5rem;
      max-width: none;
    }

    .notification-actions {
      flex-direction: column;
    }

    .action-btn {
      width: 100%;
    }
  }

  /* Dark mode */
  @media (prefers-color-scheme: dark) {
    .notification {
      background: #1f2937;
      border-color: #374151;
    }

    .notification-title {
      color: #e5e7eb;
    }

    .notification-message {
      color: #9ca3af;
    }

    .dismiss-btn:hover {
      background: #374151;
      color: #d1d5db;
    }

    .progress-bar {
      background: #4b5563;
    }

    .action-secondary {
      background: #374151;
      color: #d1d5db;
      border-color: #4b5563;
    }

    .action-secondary:hover {
      background: #4b5563;
    }
  }

  /* High contrast accessibility */
  @media (prefers-contrast: high) {
    .notification {
      border-width: 2px;
    }

    .action-btn {
      border-width: 2px;
    }
  }

  /* Reduced motion */
  @media (prefers-reduced-motion: reduce) {
    .notification {
      animation: none;
    }

    .notification-loading .notification-icon {
      animation: none;
    }

    .progress-fill {
      transition: none;
    }
  }
</style>