/*!
 * Service Worker para Protein Docking v2
 * Manejo de cache para máxima performance
 */

const version = '2.0.0';
const build = [];
const files = [];

const CACHE_NAME = `protein-docking-v2-${version}`;
const STATIC_CACHE = `static-${version}`;
const DYNAMIC_CACHE = `dynamic-${version}`;

// Assets que siempre deben estar en cache
const STATIC_ASSETS = [
  ...build, // App bundle
  ...files  // Static files
];

// Assets críticos que se cargan inmediatamente
const CRITICAL_ASSETS = [
  '/',
  '/favicon.png',
  '/manifest.json'
];

self.addEventListener('install', (event: ExtendableEvent) => {
  console.log('[SW] Installing service worker version', version);
  
  event.waitUntil(
    Promise.all([
      // Cache static assets
      caches.open(STATIC_CACHE)
        .then(cache => cache.addAll(STATIC_ASSETS)),
      
      // Cache critical assets
      caches.open(DYNAMIC_CACHE)
        .then(cache => cache.addAll(CRITICAL_ASSETS))
    ]).then(() => {
      console.log('[SW] Installation complete');
      // Force activation without waiting
      return self.skipWaiting();
    })
  );
});

self.addEventListener('activate', (event: ExtendableEvent) => {
  console.log('[SW] Activating service worker version', version);
  
  event.waitUntil(
    Promise.all([
      // Clean old caches
      caches.keys().then(cacheNames => {
        return Promise.all(
          cacheNames
            .filter(cacheName => cacheName.startsWith('protein-docking-v2-') && cacheName !== CACHE_NAME)
            .map(cacheName => {
              console.log('[SW] Deleting old cache:', cacheName);
              return caches.delete(cacheName);
            })
        );
      }),
      
      // Take control of all clients
      self.clients.claim()
    ]).then(() => {
      console.log('[SW] Activation complete');
    })
  );
});

self.addEventListener('fetch', (event: FetchEvent) => {
  const { request } = event;
  const url = new URL(request.url);
  
  // Skip non-HTTP requests
  if (!url.protocol.startsWith('http')) return;
  
  // Skip requests to external domains (API calls)
  if (url.origin !== location.origin) {
    // For API calls, use network first with timeout
    if (url.pathname.startsWith('/api/')) {
      event.respondWith(networkFirstWithTimeout(request, 5000));
      return;
    }
    return;
  }
  
  // Handle different types of requests
  if (STATIC_ASSETS.includes(url.pathname)) {
    // Static assets: cache first
    event.respondWith(cacheFirst(request, STATIC_CACHE));
  } else if (url.pathname === '/' || url.pathname.endsWith('.html')) {
    // HTML pages: network first with cache fallback
    event.respondWith(networkFirst(request, DYNAMIC_CACHE));
  } else if (url.pathname.startsWith('/_app/')) {
    // App assets: cache first
    event.respondWith(cacheFirst(request, STATIC_CACHE));
  } else {
    // Everything else: network first
    event.respondWith(networkFirst(request, DYNAMIC_CACHE));
  }
});

// Cache first strategy
async function cacheFirst(request: Request, cacheName: string): Promise<Response> {
  const cache = await caches.open(cacheName);
  const cached = await cache.match(request);
  
  if (cached) {
    console.log('[SW] Cache hit:', request.url);
    return cached;
  }
  
  try {
    const response = await fetch(request);
    if (response.status === 200) {
      cache.put(request, response.clone());
    }
    return response;
  } catch (error) {
    console.error('[SW] Network error:', error);
    throw error;
  }
}

// Network first strategy
async function networkFirst(request: Request, cacheName: string): Promise<Response> {
  const cache = await caches.open(cacheName);
  
  try {
    const response = await fetch(request);
    if (response.status === 200) {
      cache.put(request, response.clone());
    }
    return response;
  } catch (error) {
    console.log('[SW] Network failed, checking cache:', request.url);
    const cached = await cache.match(request);
    
    if (cached) {
      return cached;
    }
    
    // Fallback for navigation requests
    if (request.mode === 'navigate') {
      const fallback = await cache.match('/');
      if (fallback) return fallback;
    }
    
    throw error;
  }
}

// Network first with timeout
async function networkFirstWithTimeout(request: Request, timeout: number): Promise<Response> {
  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeout);
    
    const response = await fetch(request, {
      signal: controller.signal
    });
    
    clearTimeout(timeoutId);
    return response;
  } catch (error) {
    console.log('[SW] Network request timed out or failed:', request.url);
    throw error;
  }
}

// Handle background sync for offline requests
self.addEventListener('sync', (event: any) => {
  if (event.tag === 'background-sync') {
    console.log('[SW] Background sync triggered');
    event.waitUntil(handleBackgroundSync());
  }
});

async function handleBackgroundSync() {
  // Handle any queued requests when coming back online
  try {
    const cache = await caches.open(DYNAMIC_CACHE);
    const requests = await cache.keys();
    
    // Retry failed API requests
    for (const request of requests) {
      if (request.url.includes('/api/')) {
        try {
          await fetch(request);
          await cache.delete(request);
        } catch (error) {
          console.log('[SW] Retry failed for:', request.url);
        }
      }
    }
  } catch (error) {
    console.error('[SW] Background sync error:', error);
  }
}

// Handle push notifications
self.addEventListener('push', (event: any) => {
  if (!event.data) return;
  
  const data = event.data.json();
  const options = {
    body: data.body || 'New notification',
    icon: '/favicon.png',
    badge: '/favicon.png',
    data: data.data || {},
    actions: data.actions || []
  };
  
  event.waitUntil(
    self.registration.showNotification(data.title || 'Protein Docking', options)
  );
});

// Handle notification clicks
self.addEventListener('notificationclick', (event: any) => {
  event.notification.close();
  
  event.waitUntil(
    self.clients.openWindow(event.notification.data.url || '/')
  );
});

// Error handling
self.addEventListener('error', (error: ErrorEvent) => {
  console.error('[SW] Service worker error:', error);
});

self.addEventListener('unhandledrejection', (event: any) => {
  console.error('[SW] Unhandled promise rejection:', event.reason);
});

console.log('[SW] Service worker loaded, version:', version);