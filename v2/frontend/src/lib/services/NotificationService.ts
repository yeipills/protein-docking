/*!
 * NotificationService - Sistema de notificaciones en tiempo real
 * 
 * Features:
 * - Notificaciones toast con animaciones fluidas
 * - Integración con WebSocket para updates en tiempo real
 * - Sistema de prioridades y categorización
 * - Auto-dismiss con timers configurables
 * - Queue management para múltiples notificaciones
 * - Persistencia opcional en localStorage
 */

export type NotificationType = 'success' | 'error' | 'warning' | 'info' | 'loading';

export type NotificationPriority = 'low' | 'normal' | 'high' | 'critical';

export interface NotificationAction {
  label: string;
  action: () => void;
  style?: 'primary' | 'secondary' | 'danger';
}

export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  message: string;
  priority: NotificationPriority;
  timestamp: number;
  duration?: number; // ms, null = persistent
  actions?: NotificationAction[];
  progress?: number; // 0-100 for progress notifications
  metadata?: Record<string, any>;
  dismissed?: boolean;
  category?: string;
}

export interface NotificationConfig {
  maxNotifications: number;
  defaultDuration: number;
  enablePersistence: boolean;
  enableSound: boolean;
  position: 'top-right' | 'top-left' | 'bottom-right' | 'bottom-left' | 'top-center' | 'bottom-center';
  enableWebSocket: boolean;
  soundUrls: Record<NotificationType, string>;
}

class NotificationService {
  private notifications = new Map<string, Notification>();
  private subscribers = new Set<(notifications: Notification[]) => void>();
  private config: NotificationConfig;
  private dismissTimers = new Map<string, number>();
  private nextId = 1;

  constructor(config: Partial<NotificationConfig> = {}) {
    this.config = {
      maxNotifications: 5,
      defaultDuration: 5000,
      enablePersistence: false,
      enableSound: false,
      position: 'top-right',
      enableWebSocket: true,
      soundUrls: {
        success: '/sounds/success.mp3',
        error: '/sounds/error.mp3',
        warning: '/sounds/warning.mp3',
        info: '/sounds/info.mp3',
        loading: '/sounds/loading.mp3'
      },
      ...config
    };

    this.loadPersistedNotifications();
  }

  // Subscription management
  subscribe(callback: (notifications: Notification[]) => void): () => void {
    this.subscribers.add(callback);
    callback(this.getActiveNotifications());
    
    return () => {
      this.subscribers.delete(callback);
    };
  }

  private notify(): void {
    const activeNotifications = this.getActiveNotifications();
    this.subscribers.forEach(callback => callback(activeNotifications));
  }

  // Core notification methods
  show(notification: Omit<Notification, 'id' | 'timestamp'>): string {
    const id = this.generateId();
    const fullNotification: Notification = {
      id,
      timestamp: Date.now(),
      duration: this.config.defaultDuration,
      ...notification
    };

    this.notifications.set(id, fullNotification);
    this.enforceMaxNotifications();
    
    // Set up auto-dismiss timer
    if (fullNotification.duration && fullNotification.duration > 0) {
      this.setDismissTimer(id, fullNotification.duration);
    }

    // Play sound if enabled
    if (this.config.enableSound) {
      this.playSound(fullNotification.type);
    }

    // Persist if enabled
    if (this.config.enablePersistence) {
      this.persistNotifications();
    }

    this.notify();
    
    console.log(`🔔 Notification: [${fullNotification.type.toUpperCase()}] ${fullNotification.title}`);
    
    return id;
  }

  // Convenience methods
  success(title: string, message: string = '', options: Partial<Notification> = {}): string {
    return this.show({
      type: 'success',
      title,
      message,
      priority: 'normal',
      ...options
    });
  }

  error(title: string, message: string = '', options: Partial<Notification> = {}): string {
    return this.show({
      type: 'error',
      title,
      message,
      priority: 'high',
      duration: 8000, // Errors stay longer
      ...options
    });
  }

  warning(title: string, message: string = '', options: Partial<Notification> = {}): string {
    return this.show({
      type: 'warning',
      title,
      message,
      priority: 'normal',
      duration: 6000,
      ...options
    });
  }

  info(title: string, message: string = '', options: Partial<Notification> = {}): string {
    return this.show({
      type: 'info',
      title,
      message,
      priority: 'low',
      ...options
    });
  }

  loading(title: string, message: string = '', options: Partial<Notification> = {}): string {
    return this.show({
      type: 'loading',
      title,
      message,
      priority: 'normal',
      duration: null, // Loading notifications are persistent
      ...options
    });
  }

  // Progress notifications
  showProgress(title: string, message: string = '', initialProgress: number = 0): string {
    return this.show({
      type: 'loading',
      title,
      message,
      priority: 'normal',
      duration: null,
      progress: initialProgress
    });
  }

  updateProgress(id: string, progress: number, message?: string): void {
    const notification = this.notifications.get(id);
    if (notification) {
      notification.progress = progress;
      if (message) notification.message = message;
      
      // Auto-complete at 100%
      if (progress >= 100) {
        setTimeout(() => {
          this.complete(id, 'Process completed successfully!');
        }, 500);
      }
      
      this.notify();
    }
  }

  complete(id: string, message?: string): void {
    const notification = this.notifications.get(id);
    if (notification) {
      notification.type = 'success';
      notification.progress = undefined;
      if (message) notification.message = message;
      notification.duration = 3000;
      
      this.setDismissTimer(id, 3000);
      this.notify();
    }
  }

  // Job-specific notifications
  jobStarted(jobId: string, type: string = 'Docking Job'): string {
    return this.loading(
      `${type} Started`,
      `Job ${jobId} is now processing...`,
      {
        category: 'job',
        metadata: { jobId, status: 'started' }
      }
    );
  }

  jobProgress(notificationId: string, jobId: string, progress: number, step?: string): void {
    const message = step ? `${step} (${progress.toFixed(1)}%)` : `Progress: ${progress.toFixed(1)}%`;
    this.updateProgress(notificationId, progress, message);
  }

  jobCompleted(notificationId: string, jobId: string, resultCount?: number): void {
    const message = resultCount 
      ? `Generated ${resultCount} poses successfully!`
      : 'Job completed successfully!';
    this.complete(notificationId, message);
  }

  jobFailed(jobId: string, error: string): string {
    return this.error(
      'Job Failed',
      `Job ${jobId} failed: ${error}`,
      {
        category: 'job',
        metadata: { jobId, status: 'failed', error },
        actions: [
          {
            label: 'Retry',
            action: () => this.retryJob(jobId),
            style: 'primary'
          },
          {
            label: 'View Details',
            action: () => this.viewJobDetails(jobId),
            style: 'secondary'
          }
        ]
      }
    );
  }

  // File operation notifications
  fileUploaded(filename: string, fileId: string): string {
    return this.success(
      'File Uploaded',
      `${filename} uploaded successfully`,
      {
        category: 'file',
        metadata: { filename, fileId },
        actions: [
          {
            label: 'View File',
            action: () => this.viewFile(fileId),
            style: 'primary'
          }
        ]
      }
    );
  }

  fileUploadFailed(filename: string, error: string): string {
    return this.error(
      'Upload Failed',
      `Failed to upload ${filename}: ${error}`,
      {
        category: 'file',
        metadata: { filename, error }
      }
    );
  }

  // System notifications
  connectionLost(): string {
    return this.warning(
      'Connection Lost',
      'Reconnecting to server...',
      {
        category: 'system',
        duration: null,
        metadata: { type: 'connection' }
      }
    );
  }

  connectionRestored(): string {
    // First dismiss any connection lost notifications
    this.dismissByCategory('system');
    
    return this.success(
      'Connection Restored',
      'Successfully reconnected to server',
      {
        category: 'system',
        duration: 3000
      }
    );
  }

  // Management methods
  dismiss(id: string): void {
    const notification = this.notifications.get(id);
    if (notification) {
      notification.dismissed = true;
      this.clearDismissTimer(id);
      this.notifications.delete(id);
      
      if (this.config.enablePersistence) {
        this.persistNotifications();
      }
      
      this.notify();
    }
  }

  dismissAll(): void {
    this.notifications.clear();
    this.dismissTimers.forEach(timer => clearTimeout(timer));
    this.dismissTimers.clear();
    
    if (this.config.enablePersistence) {
      this.persistNotifications();
    }
    
    this.notify();
  }

  dismissByType(type: NotificationType): void {
    const toRemove: string[] = [];
    this.notifications.forEach((notification, id) => {
      if (notification.type === type) {
        toRemove.push(id);
      }
    });
    
    toRemove.forEach(id => this.dismiss(id));
  }

  dismissByCategory(category: string): void {
    const toRemove: string[] = [];
    this.notifications.forEach((notification, id) => {
      if (notification.category === category) {
        toRemove.push(id);
      }
    });
    
    toRemove.forEach(id => this.dismiss(id));
  }

  // Utility methods
  private getActiveNotifications(): Notification[] {
    return Array.from(this.notifications.values())
      .filter(n => !n.dismissed)
      .sort((a, b) => {
        // Sort by priority first, then by timestamp
        const priorityOrder = { critical: 4, high: 3, normal: 2, low: 1 };
        const priorityDiff = priorityOrder[b.priority] - priorityOrder[a.priority];
        
        return priorityDiff !== 0 ? priorityDiff : b.timestamp - a.timestamp;
      });
  }

  private enforceMaxNotifications(): void {
    const active = this.getActiveNotifications();
    if (active.length > this.config.maxNotifications) {
      // Remove oldest low-priority notifications
      const toRemove = active
        .filter(n => n.priority === 'low')
        .slice(this.config.maxNotifications)
        .map(n => n.id);
      
      toRemove.forEach(id => this.dismiss(id));
    }
  }

  private setDismissTimer(id: string, duration: number): void {
    this.clearDismissTimer(id);
    
    const timer = window.setTimeout(() => {
      this.dismiss(id);
    }, duration);
    
    this.dismissTimers.set(id, timer);
  }

  private clearDismissTimer(id: string): void {
    const timer = this.dismissTimers.get(id);
    if (timer) {
      clearTimeout(timer);
      this.dismissTimers.delete(id);
    }
  }

  private generateId(): string {
    return `notification-${this.nextId++}-${Date.now()}`;
  }

  private async playSound(type: NotificationType): Promise<void> {
    try {
      const soundUrl = this.config.soundUrls[type];
      if (soundUrl) {
        const audio = new Audio(soundUrl);
        audio.volume = 0.3; // Keep it subtle
        await audio.play();
      }
    } catch (error) {
      // Silently fail if sound can't play
    }
  }

  private persistNotifications(): void {
    if (typeof localStorage !== 'undefined') {
      const serializable = Array.from(this.notifications.values()).map(n => ({
        ...n,
        // Don't persist actions (functions can't be serialized)
        actions: undefined
      }));
      
      localStorage.setItem('notifications', JSON.stringify(serializable));
    }
  }

  private loadPersistedNotifications(): void {
    if (typeof localStorage !== 'undefined' && this.config.enablePersistence) {
      try {
        const stored = localStorage.getItem('notifications');
        if (stored) {
          const notifications: Notification[] = JSON.parse(stored);
          
          // Only load recent notifications (last 24 hours)
          const cutoff = Date.now() - (24 * 60 * 60 * 1000);
          
          notifications
            .filter(n => n.timestamp > cutoff && !n.dismissed)
            .forEach(n => {
              this.notifications.set(n.id, n);
              
              // Restart timers for auto-dismiss
              if (n.duration && n.duration > 0) {
                const elapsed = Date.now() - n.timestamp;
                const remaining = n.duration - elapsed;
                
                if (remaining > 0) {
                  this.setDismissTimer(n.id, remaining);
                } else {
                  this.dismiss(n.id);
                }
              }
            });
        }
      } catch (error) {
        console.warn('Failed to load persisted notifications:', error);
      }
    }
  }

  // Action handlers (to be implemented by the application)
  private retryJob(jobId: string): void {
    console.log('🔄 Retry job:', jobId);
    // Implementation depends on job management system
  }

  private viewJobDetails(jobId: string): void {
    console.log('👁️ View job details:', jobId);
    // Implementation depends on routing system
  }

  private viewFile(fileId: string): void {
    console.log('📄 View file:', fileId);
    // Implementation depends on file management system
  }

  // Configuration
  updateConfig(newConfig: Partial<NotificationConfig>): void {
    this.config = { ...this.config, ...newConfig };
  }

  getConfig(): NotificationConfig {
    return { ...this.config };
  }

  // Statistics
  getStats(): {
    total: number;
    active: number;
    byType: Record<NotificationType, number>;
    byPriority: Record<NotificationPriority, number>;
  } {
    const active = this.getActiveNotifications();
    
    const byType: Record<NotificationType, number> = {
      success: 0, error: 0, warning: 0, info: 0, loading: 0
    };
    
    const byPriority: Record<NotificationPriority, number> = {
      low: 0, normal: 0, high: 0, critical: 0
    };
    
    active.forEach(n => {
      byType[n.type]++;
      byPriority[n.priority]++;
    });
    
    return {
      total: this.notifications.size,
      active: active.length,
      byType,
      byPriority
    };
  }
}

// Singleton instance
export const notificationService = new NotificationService();

// Export for custom instances
export { NotificationService };