import { useState, useCallback } from 'react';
import { apiClient } from '@/utils/httpClient';

interface UseApiState<T> {
  data: T | null;
  loading: boolean;
  error: Error | null;
}

interface UseApiOptions {
  onSuccess?: (data: any) => void;
  onError?: (error: Error) => void;
  retry?: {
    maxRetries?: number;
    retryDelay?: number;
  };
}

/**
 * Custom hook for API calls with retry logic
 * Provides loading, error states and automatic retry on failure
 */
export function useApi<T = any>(options: UseApiOptions = {}) {
  const [state, setState] = useState<UseApiState<T>>({
    data: null,
    loading: false,
    error: null
  });

  const execute = useCallback(
    async (
      method: 'get' | 'post' | 'put' | 'patch' | 'delete',
      path: string,
      data?: any
    ): Promise<T | null> => {
      setState({ data: null, loading: true, error: null });

      try {
        let result: T;

        switch (method) {
          case 'get':
            result = await apiClient.get<T>(path, { retry: options.retry });
            break;
          case 'post':
            result = await apiClient.post<T>(path, data, { retry: options.retry });
            break;
          case 'put':
            result = await apiClient.put<T>(path, data, { retry: options.retry });
            break;
          case 'patch':
            result = await apiClient.patch<T>(path, data, { retry: options.retry });
            break;
          case 'delete':
            result = await apiClient.delete<T>(path, { retry: options.retry });
            break;
          default:
            throw new Error(`Unsupported method: ${method}`);
        }

        setState({ data: result, loading: false, error: null });

        if (options.onSuccess) {
          options.onSuccess(result);
        }

        return result;
      } catch (error) {
        const err = error as Error;
        setState({ data: null, loading: false, error: err });

        if (options.onError) {
          options.onError(err);
        }

        return null;
      }
    },
    [options]
  );

  const get = useCallback(
    (path: string) => execute('get', path),
    [execute]
  );

  const post = useCallback(
    (path: string, data?: any) => execute('post', path, data),
    [execute]
  );

  const put = useCallback(
    (path: string, data?: any) => execute('put', path, data),
    [execute]
  );

  const patch = useCallback(
    (path: string, data?: any) => execute('patch', path, data),
    [execute]
  );

  const del = useCallback(
    (path: string) => execute('delete', path),
    [execute]
  );

  const reset = useCallback(() => {
    setState({ data: null, loading: false, error: null });
  }, []);

  return {
    ...state,
    get,
    post,
    put,
    patch,
    delete: del,
    reset
  };
}

/**
 * Hook for GET requests
 */
export function useApiGet<T = any>(path: string, options: UseApiOptions = {}) {
  const api = useApi<T>(options);

  const fetch = useCallback(() => {
    return api.get(path);
  }, [api, path]);

  return {
    data: api.data,
    loading: api.loading,
    error: api.error,
    fetch,
    reset: api.reset
  };
}

/**
 * Hook for POST requests
 */
export function useApiPost<T = any>(path: string, options: UseApiOptions = {}) {
  const api = useApi<T>(options);

  const submit = useCallback(
    (data?: any) => {
      return api.post(path, data);
    },
    [api, path]
  );

  return {
    data: api.data,
    loading: api.loading,
    error: api.error,
    submit,
    reset: api.reset
  };
}

/**
 * Hook for mutation operations (POST, PUT, PATCH, DELETE)
 */
export function useApiMutation<T = any>(options: UseApiOptions = {}) {
  const api = useApi<T>(options);

  const mutate = useCallback(
    async (
      method: 'post' | 'put' | 'patch' | 'delete',
      path: string,
      data?: any
    ) => {
      switch (method) {
        case 'post':
          return api.post(path, data);
        case 'put':
          return api.put(path, data);
        case 'patch':
          return api.patch(path, data);
        case 'delete':
          return api.delete(path);
        default:
          throw new Error(`Unsupported method: ${method}`);
      }
    },
    [api]
  );

  return {
    data: api.data,
    loading: api.loading,
    error: api.error,
    mutate,
    reset: api.reset
  };
}

export default useApi;
