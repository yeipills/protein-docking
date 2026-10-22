// WebSocket Service - Real-time comunicación con backend Rust
// Job progress updates, connection management, reconnection automática

import type { JobProgressUpdate, ConnectionState } from '../types/unified.js';

// Re-export para compatibilidad
export type { JobProgressUpdate, ConnectionState };

export interface WebSocketMessage {
  type: 'Subscribe' | 'Unsubscribe' | 'Ping' | 'Pong' | 'JobProgress' | 'Error' | 'Subscribed' | 'Unsubscribed';
  data?: any;
}

// Extender ConnectionState con campos adicionales
export interface ExtendedConnectionState extends ConnectionState {
  connecting: boolean;
  error?: string;
  lastPing?: number;
  reconnectAttempts: number;
}

type JobProgressCallback = (update: JobProgressUpdate) => void;
type ConnectionCallback = (state: ConnectionState) => void;
type ErrorCallback = (error: string) => void;

class WebSocketService {
  private ws: WebSocket | null = null;
  private baseURL: string;
  private reconnectTimer: number | null = null;
  private pingTimer: number | null = null;
  private maxReconnectAttempts = 10;
  private reconnectDelay = 1000;
  private pingInterval = 30000; // 30 seconds
  
  // State
  private connectionState: ExtendedConnectionState = {
    connected: false,
    connecting: false,
    reconnectAttempts: 0,
  };
  
  // Subscriptions and callbacks
  private jobSubscriptions = new Set<string>();
  private jobProgressCallbacks = new Map<string, JobProgressCallback[]>();
  private globalProgressCallbacks: JobProgressCallback[] = [];
  private connectionCallbacks: ConnectionCallback[] = [];
  private errorCallbacks: ErrorCallback[] = [];

  constructor(baseURL?: string) {
    this.baseURL = baseURL || this.detectWebSocketURL();
  }

  private detectWebSocketURL(): string {
    if (typeof window !== 'undefined') {
      const hostname = window.location.hostname;
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      
      // Development: backend on port 8080
      if (hostname === 'localhost' || hostname === '127.0.0.1') {
        return `${protocol}//${hostname}:8080`;
      }
      
      // Production: same domain
      return `${protocol}//${hostname}`;
    }
    
    return 'ws://localhost:8080';
  }

  // Connection management
  connect(): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      console.log('🔌 WebSocket already connected');
      return;
    }

    if (this.connectionState.connecting) {
      console.log('🔌 WebSocket connection already in progress');
      return;
    }

    this.updateConnectionState({ connecting: true, error: undefined });
    
    try {
      const wsURL = `${this.baseURL}/ws/jobs`;
      console.log(`🔌 Connecting to WebSocket: ${wsURL}`);
      
      this.ws = new WebSocket(wsURL);
      this.setupEventListeners();
      
    } catch (error) {
      console.error('🚨 WebSocket connection failed:', error);
      this.updateConnectionState({ 
        connecting: false, 
        error: error instanceof Error ? error.message : 'Connection failed' 
      });
      this.scheduleReconnect();
    }
  }

  disconnect(): void {
    console.log('🔌 Disconnecting WebSocket');
    
    this.clearTimers();
    
    if (this.ws) {
      this.ws.close(1000, 'Client disconnect');
      this.ws = null;
    }
    
    this.updateConnectionState({ 
      connected: false, 
      connecting: false, 
      reconnectAttempts: 0 
    });
  }

  private setupEventListeners(): void {
    if (!this.ws) return;

    this.ws.onopen = () => {
      console.log('✅ WebSocket connected');
      this.updateConnectionState({ 
        connected: true, 
        connecting: false, 
        error: undefined,
        reconnectAttempts: 0 
      });
      
      this.startPingTimer();
      this.resubscribeToJobs();
    };

    this.ws.onclose = (event) => {
      console.log(`🔌 WebSocket closed: ${event.code} - ${event.reason}`);
      this.updateConnectionState({ 
        connected: false, 
        connecting: false 
      });
      
      this.clearTimers();
      
      // Auto-reconnect unless it was a clean close
      if (event.code !== 1000) {
        this.scheduleReconnect();
      }
    };

    this.ws.onerror = (error) => {
      console.error('🚨 WebSocket error:', error);
      this.updateConnectionState({ 
        connecting: false, 
        error: 'Connection error' 
      });
      this.notifyError('WebSocket connection error');
    };

    this.ws.onmessage = (event) => {
      try {
        const message: WebSocketMessage = JSON.parse(event.data);
        this.handleMessage(message);
      } catch (error) {
        console.error('🚨 Failed to parse WebSocket message:', error);
        this.notifyError('Invalid message format');
      }
    };
  }

  private handleMessage(message: WebSocketMessage): void {
    switch (message.type) {
      case 'JobProgress':
        this.handleJobProgress(message.data);
        break;
        
      case 'Subscribed':
        console.log(`✅ Subscribed to job: ${message.data.job_id}`);
        break;
        
      case 'Unsubscribed':
        console.log(`❌ Unsubscribed from job: ${message.data.job_id}`);
        break;
        
      case 'Pong':
        this.updateConnectionState({ lastPing: Date.now() });
        break;
        
      case 'Error':
        console.error('🚨 WebSocket server error:', message.data.message);
        this.notifyError(message.data.message);
        break;
        
      default:
        console.warn('🤔 Unknown WebSocket message type:', message.type);
    }
  }

  private handleJobProgress(update: JobProgressUpdate): void {
    console.log(`📊 Job progress update: ${update.job_id} - ${update.progress_percent}%`);
    
    // Notify global callbacks
    this.globalProgressCallbacks.forEach(callback => {
      try {
        callback(update);
      } catch (error) {
        console.error('Error in global progress callback:', error);
      }
    });
    
    // Notify job-specific callbacks
    const jobCallbacks = this.jobProgressCallbacks.get(update.job_id);
    if (jobCallbacks) {
      jobCallbacks.forEach(callback => {
        try {
          callback(update);
        } catch (error) {
          console.error(`Error in job ${update.job_id} progress callback:`, error);
        }
      });
    }
  }

  private sendMessage(message: WebSocketMessage): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      console.warn('🚨 Cannot send message: WebSocket not connected');
      throw new Error('WebSocket not connected');
    }
  }

  private startPingTimer(): void {
    this.clearTimers();
    this.pingTimer = window.setInterval(() => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.sendMessage({ type: 'Ping' });
      }
    }, this.pingInterval);
  }

  private clearTimers(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    
    if (this.pingTimer) {
      clearInterval(this.pingTimer);
      this.pingTimer = null;
    }
  }

  private scheduleReconnect(): void {
    if (this.connectionState.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('🚨 Max reconnection attempts reached');
      this.updateConnectionState({ error: 'Max reconnection attempts reached' });
      return;
    }

    const delay = this.reconnectDelay * Math.pow(2, this.connectionState.reconnectAttempts);
    console.log(`🔄 Scheduling reconnect in ${delay}ms (attempt ${this.connectionState.reconnectAttempts + 1})`);
    
    this.reconnectTimer = window.setTimeout(() => {
      this.updateConnectionState({ reconnectAttempts: this.connectionState.reconnectAttempts + 1 });
      this.connect();
    }, delay);
  }

  private resubscribeToJobs(): void {
    // Resubscribe to all jobs after reconnection
    this.jobSubscriptions.forEach(jobId => {
      this.sendMessage({
        type: 'Subscribe',
        data: { job_id: jobId }
      });
    });
  }

  private updateConnectionState(updates: Partial<ExtendedConnectionState>): void {
    this.connectionState = { ...this.connectionState, ...updates };
    this.connectionCallbacks.forEach(callback => {
      try {
        callback(this.connectionState);
      } catch (error) {
        console.error('Error in connection callback:', error);
      }
    });
  }

  private notifyError(error: string): void {
    this.errorCallbacks.forEach(callback => {
      try {
        callback(error);
      } catch (err) {
        console.error('Error in error callback:', err);
      }
    });
  }

  // Public API

  // Job subscriptions
  subscribeToJob(jobId: string, callback?: JobProgressCallback): void {
    console.log(`📋 Subscribing to job: ${jobId}`);
    
    this.jobSubscriptions.add(jobId);
    
    if (callback) {
      const callbacks = this.jobProgressCallbacks.get(jobId) || [];
      callbacks.push(callback);
      this.jobProgressCallbacks.set(jobId, callbacks);
    }
    
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.sendMessage({
        type: 'Subscribe',
        data: { job_id: jobId }
      });
    }
  }

  unsubscribeFromJob(jobId: string, callback?: JobProgressCallback): void {
    console.log(`📋 Unsubscribing from job: ${jobId}`);
    
    this.jobSubscriptions.delete(jobId);
    
    if (callback) {
      const callbacks = this.jobProgressCallbacks.get(jobId) || [];
      const filteredCallbacks = callbacks.filter(cb => cb !== callback);
      
      if (filteredCallbacks.length > 0) {
        this.jobProgressCallbacks.set(jobId, filteredCallbacks);
      } else {
        this.jobProgressCallbacks.delete(jobId);
      }
    } else {
      this.jobProgressCallbacks.delete(jobId);
    }
    
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.sendMessage({
        type: 'Unsubscribe',
        data: { job_id: jobId }
      });
    }
  }

  // Global callbacks
  onJobProgress(callback: JobProgressCallback): () => void {
    this.globalProgressCallbacks.push(callback);
    
    // Return unsubscribe function
    return () => {
      const index = this.globalProgressCallbacks.indexOf(callback);
      if (index > -1) {
        this.globalProgressCallbacks.splice(index, 1);
      }
    };
  }

  onConnectionChange(callback: ConnectionCallback): () => void {
    this.connectionCallbacks.push(callback);
    
    // Immediately call with current state
    callback(this.connectionState);
    
    // Return unsubscribe function
    return () => {
      const index = this.connectionCallbacks.indexOf(callback);
      if (index > -1) {
        this.connectionCallbacks.splice(index, 1);
      }
    };
  }

  onError(callback: ErrorCallback): () => void {
    this.errorCallbacks.push(callback);
    
    // Return unsubscribe function
    return () => {
      const index = this.errorCallbacks.indexOf(callback);
      if (index > -1) {
        this.errorCallbacks.splice(index, 1);
      }
    };
  }

  // State getters
  isConnected(): boolean {
    return this.connectionState.connected;
  }

  isConnecting(): boolean {
    return this.connectionState.connecting;
  }

  getConnectionState(): ConnectionState {
    return { ...this.connectionState };
  }

  // Manual ping
  ping(): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.sendMessage({ type: 'Ping' });
    }
  }

  // Update base URL
  updateBaseURL(newBaseURL: string): void {
    const wasConnected = this.isConnected();
    
    this.disconnect();
    this.baseURL = newBaseURL;
    
    if (wasConnected) {
      // Reconnect with new URL
      setTimeout(() => this.connect(), 100);
    }
  }

  // Get current base URL
  getBaseURL(): string {
    return this.baseURL;
  }
}

// Singleton instance
export const webSocketService = new WebSocketService();

// Export for testing with custom URLs
export { WebSocketService };