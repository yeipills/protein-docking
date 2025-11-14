/**
 * HTTP Client with Retry Logic
 * Provides automatic retry for failed requests with exponential backoff
 */

interface RetryConfig {
  maxRetries?: number;
  retryDelay?: number;
  retryOn?: number[]; // HTTP status codes to retry on
  shouldRetry?: (error: any, attemptNumber: number) => boolean;
}

interface FetchOptions extends RequestInit {
  retry?: RetryConfig;
}

const DEFAULT_RETRY_CONFIG: Required<RetryConfig> = {
  maxRetries: 3,
  retryDelay: 1000, // 1 second
  retryOn: [408, 429, 500, 502, 503, 504], // Timeout, Rate limit, Server errors
  shouldRetry: (error: any, attemptNumber: number) => true
};

/**
 * Sleep utility for delays
 */
const sleep = (ms: number): Promise<void> => {
  return new Promise(resolve => setTimeout(resolve, ms));
};

/**
 * Calculate exponential backoff delay
 */
const getRetryDelay = (attemptNumber: number, baseDelay: number): number => {
  // Exponential backoff: baseDelay * 2^attemptNumber with jitter
  const exponentialDelay = baseDelay * Math.pow(2, attemptNumber);
  const jitter = Math.random() * 1000; // Add up to 1 second of jitter
  return Math.min(exponentialDelay + jitter, 30000); // Max 30 seconds
};

/**
 * Enhanced fetch with retry logic
 */
export async function fetchWithRetry(
  url: string,
  options: FetchOptions = {}
): Promise<Response> {
  const { retry = {}, ...fetchOptions } = options;
  const retryConfig = { ...DEFAULT_RETRY_CONFIG, ...retry };

  let lastError: Error | null = null;
  let response: Response | null = null;

  for (let attempt = 0; attempt <= retryConfig.maxRetries; attempt++) {
    try {
      // Add request ID header for tracing
      const headers = new Headers(fetchOptions.headers);
      if (!headers.has('X-Request-ID')) {
        headers.set('X-Request-ID', crypto.randomUUID());
      }

      // Make the request
      response = await fetch(url, {
        ...fetchOptions,
        headers
      });

      // Check if response is successful
      if (response.ok) {
        return response;
      }

      // Check if we should retry based on status code
      if (!retryConfig.retryOn.includes(response.status)) {
        return response; // Don't retry, return the error response
      }

      // For non-2xx responses that we want to retry
      lastError = new Error(`HTTP ${response.status}: ${response.statusText}`);

    } catch (error) {
      // Network error or other fetch error
      lastError = error as Error;
    }

    // Check if we should retry
    const shouldRetry = retryConfig.shouldRetry(lastError, attempt);
    const isLastAttempt = attempt === retryConfig.maxRetries;

    if (!shouldRetry || isLastAttempt) {
      break;
    }

    // Calculate delay and wait before retrying
    const delay = getRetryDelay(attempt, retryConfig.retryDelay);
    console.warn(`Request failed (attempt ${attempt + 1}/${retryConfig.maxRetries + 1}). Retrying in ${delay}ms...`);
    await sleep(delay);
  }

  // If we have a response (even an error one), return it
  if (response) {
    return response;
  }

  // Otherwise throw the last error
  throw lastError || new Error('Request failed after retries');
}

/**
 * HTTP Client class with convenient methods
 */
export class HttpClient {
  private baseURL: string;
  private defaultHeaders: HeadersInit;
  private defaultRetryConfig: RetryConfig;

  constructor(
    baseURL: string = '',
    defaultHeaders: HeadersInit = {},
    defaultRetryConfig: RetryConfig = {}
  ) {
    this.baseURL = baseURL;
    this.defaultHeaders = defaultHeaders;
    this.defaultRetryConfig = { ...DEFAULT_RETRY_CONFIG, ...defaultRetryConfig };
  }

  private async request<T>(
    path: string,
    options: FetchOptions = {}
  ): Promise<T> {
    const url = this.baseURL + path;
    const headers = new Headers({
      'Content-Type': 'application/json',
      ...this.defaultHeaders,
      ...options.headers
    });

    const retryConfig = {
      ...this.defaultRetryConfig,
      ...options.retry
    };

    const response = await fetchWithRetry(url, {
      ...options,
      headers,
      retry: retryConfig
    });

    // Handle errors
    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        errorData.message ||
        errorData.detail ||
        `HTTP ${response.status}: ${response.statusText}`
      );
    }

    // Parse JSON response
    const contentType = response.headers.get('content-type');
    if (contentType && contentType.includes('application/json')) {
      return response.json();
    }

    // Return response as is for non-JSON responses
    return response as any;
  }

  async get<T>(path: string, options?: FetchOptions): Promise<T> {
    return this.request<T>(path, { ...options, method: 'GET' });
  }

  async post<T>(path: string, data?: any, options?: FetchOptions): Promise<T> {
    return this.request<T>(path, {
      ...options,
      method: 'POST',
      body: data ? JSON.stringify(data) : undefined
    });
  }

  async put<T>(path: string, data?: any, options?: FetchOptions): Promise<T> {
    return this.request<T>(path, {
      ...options,
      method: 'PUT',
      body: data ? JSON.stringify(data) : undefined
    });
  }

  async patch<T>(path: string, data?: any, options?: FetchOptions): Promise<T> {
    return this.request<T>(path, {
      ...options,
      method: 'PATCH',
      body: data ? JSON.stringify(data) : undefined
    });
  }

  async delete<T>(path: string, options?: FetchOptions): Promise<T> {
    return this.request<T>(path, { ...options, method: 'DELETE' });
  }

  setAuthToken(token: string | null) {
    if (token) {
      this.defaultHeaders = {
        ...this.defaultHeaders,
        Authorization: `Bearer ${token}`
      };
    } else {
      const headers = { ...this.defaultHeaders };
      delete (headers as any).Authorization;
      this.defaultHeaders = headers;
    }
  }
}

// Create default API client instance
const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:5000';

export const apiClient = new HttpClient(API_BASE_URL, {}, {
  maxRetries: 3,
  retryDelay: 1000,
  retryOn: [408, 429, 500, 502, 503, 504],
  shouldRetry: (error: any, attemptNumber: number) => {
    // Don't retry on authentication errors
    if (error.message && error.message.includes('401')) {
      return false;
    }
    // Don't retry on validation errors
    if (error.message && error.message.includes('400')) {
      return false;
    }
    return true;
  }
});

export default apiClient;
