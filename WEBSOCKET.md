# WebSocket Protocol Documentation

## Overview

The Protein Docking Platform uses Socket.IO for real-time bidirectional communication between the server and clients. This enables live job status updates, progress tracking, and instant notifications.

## Architecture

```
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│   Frontend  │◄───────►│ Socket Server│◄───────►│   Backend   │
│  (React)    │  WS/WSS │ (Flask-SocketIO)│   │  (FastAPI)  │
└─────────────┘         └──────────────┘         └─────────────┘
                               │
                               │
                        ┌──────▼──────┐
                        │   Celery    │
                        │   Workers   │
                        └─────────────┘
```

**Components:**
- **Frontend**: React client using `socket.io-client`
- **Socket Server**: Flask-SocketIO server (port 8080)
- **Backend**: FastAPI server that triggers socket events
- **Celery Workers**: Background workers that emit progress updates

---

## Connection

### Establishing Connection

#### Client-Side (Frontend)

```typescript
import io from 'socket.io-client'

const socket = io('http://localhost:5000', {
  auth: {
    token: accessToken  // JWT access token
  },
  transports: ['websocket', 'polling']
})

socket.on('connect', () => {
  console.log('Connected:', socket.id)
})
```

#### Server-Side Authentication

The socket server validates the JWT token on connection:

```python
@socketio.on('connect')
def handle_connect(auth):
    # Validate JWT token from auth.token
    payload = decode_token(auth['token'])
    if not payload:
        return False  # Reject connection

    user_id = payload.get('user_id')
    username = payload.get('username')

    # Join user-specific room
    join_room(f"user_{user_id}")

    emit('connected', {
        'message': 'Successfully connected',
        'user_id': user_id,
        'username': username
    })
```

---

## Events

### Client → Server Events

#### 1. `connect`

Automatically sent by Socket.IO when establishing connection.

**Payload:**
```json
{
  "auth": {
    "token": "JWT_ACCESS_TOKEN"
  }
}
```

**Response:**
- `connected` event with user information
- OR connection rejected (401)

---

#### 2. `ping`

Heartbeat to keep connection alive.

**Payload:** None

**Response:**
```json
{
  "timestamp": "2025-11-15T10:30:00.000Z"
}
```

**Example:**
```typescript
socket.emit('ping')

socket.on('pong', (data) => {
  console.log('Server time:', data.timestamp)
})
```

---

#### 3. `disconnect`

Automatically sent when client disconnects.

**Server Action:**
- Remove from connected users
- Leave user room
- Log disconnection

---

### Server → Client Events

#### 1. `connected`

Sent immediately after successful authentication.

**Payload:**
```json
{
  "message": "Successfully connected to Protein Docking Platform",
  "user_id": 123,
  "username": "john_doe"
}
```

---

#### 2. `job_started`

Emitted when a job begins processing.

**Payload:**
```json
{
  "job_id": 456,
  "job_type": "part_one",
  "message": "Processing started for part_one"
}
```

**Trigger:** Called from Celery worker when job starts

```python
from socket_server.app import notify_job_started

notify_job_started(
    user_id=123,
    job_id=456,
    job_type='part_one'
)
```

---

#### 3. `job_progress`

Real-time progress updates during job execution.

**Payload:**
```json
{
  "job_id": 456,
  "progress": 45,
  "message": "Processing centroids... (2500/5000)"
}
```

**Usage in Celery:**
```python
from socket_server.app import notify_job_progress

# In Celery task
for i, centroid in enumerate(centroids):
    # Process centroid...

    if i % 100 == 0:
        progress = int((i / total) * 100)
        notify_job_progress(
            user_id=user_id,
            job_id=job_id,
            progress=progress,
            message=f"Processing centroids... ({i}/{total})"
        )
```

---

#### 4. `job_completed`

Sent when job finishes successfully.

**Payload:**
```json
{
  "job_id": 456,
  "job_type": "part_one",
  "message": "Processing completed for part_one",
  "output_files": [
    "/uploads/user_123/protein_456/protein_CRtotales.txt",
    "/uploads/user_123/protein_456/protein_rayos_contexto.txt"
  ]
}
```

**Trigger:**
```python
from socket_server.app import notify_job_completed

notify_job_completed(
    user_id=123,
    job_id=456,
    job_type='part_one',
    output_files=['/path/to/file1.txt', '/path/to/file2.txt']
)
```

---

#### 5. `job_failed`

Sent when job encounters an error.

**Payload:**
```json
{
  "job_id": 456,
  "error": "File not found: protein.stl"
}
```

**Trigger:**
```python
from socket_server.app import notify_job_failed

notify_job_failed(
    user_id=123,
    job_id=456,
    error_message="File not found: protein.stl"
)
```

---

## User Rooms

Each authenticated user is assigned to a private room: `user_{user_id}`

**Purpose:**
- Send notifications only to specific users
- Prevent leaking information to other users
- Enable multi-session support (same user, multiple tabs)

**Room Membership:**
- **Join:** Automatically on `connect`
- **Leave:** Automatically on `disconnect`

**Emitting to Room:**
```python
socketio.emit('job_completed', data, room=f"user_{user_id}")
```

---

## Frontend Implementation

### React Hook (`useSocket`)

```typescript
import { useEffect } from 'react'
import { useAuthStore } from '@/store/authStore'
import { connectSocket, disconnectSocket } from '@/services/socket'

export function useSocket() {
  const token = useAuthStore((state) => state.accessToken)
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated)

  useEffect(() => {
    if (isAuthenticated && token) {
      // Connect with JWT token
      connectSocket(token)

      // Setup event listeners
      socket.on('job_started', (data) => {
        console.log('Job started:', data)
        // Update UI state
      })

      socket.on('job_progress', (data) => {
        console.log('Progress:', data.progress)
        // Update progress bar
      })

      socket.on('job_completed', (data) => {
        console.log('Job completed:', data)
        // Show success notification
        // Invalidate query cache to refetch
      })

      socket.on('job_failed', (data) => {
        console.error('Job failed:', data.error)
        // Show error notification
      })

      return () => {
        disconnectSocket()
      }
    }
  }, [isAuthenticated, token])
}
```

### Socket Service (`services/socket.ts`)

```typescript
import io, { Socket } from 'socket.io-client'

let socket: Socket | null = null

export function connectSocket(token: string) {
  socket = io(import.meta.env.VITE_SOCKET_URL, {
    auth: { token },
    transports: ['websocket', 'polling'],
    reconnection: true,
    reconnectionDelay: 1000,
    reconnectionAttempts: 5
  })

  socket.on('connect', () => {
    console.log('Socket connected')
  })

  socket.on('disconnect', (reason) => {
    console.log('Socket disconnected:', reason)
  })

  socket.on('connect_error', (error) => {
    console.error('Socket connection error:', error)
  })

  return socket
}

export function disconnectSocket() {
  if (socket) {
    socket.disconnect()
    socket = null
  }
}

export function getSocket() {
  return socket
}
```

---

## Error Handling

### Connection Errors

**401 Unauthorized:**
```typescript
socket.on('connect_error', (error) => {
  if (error.message === 'Invalid token') {
    // Token expired or invalid
    // Redirect to login
    logout()
  }
})
```

**Network Errors:**
```typescript
socket.on('disconnect', (reason) => {
  if (reason === 'io server disconnect') {
    // Server intentionally disconnected
    // Re-authenticate may be required
  }
  if (reason === 'ping timeout') {
    // Connection lost
    // Socket.IO will auto-reconnect
  }
})
```

---

## Security

### Authentication

✅ **JWT Validation on Connect:**
- Token validated using same secret as REST API
- Invalid tokens reject connection
- Expired tokens require refresh

### Authorization

✅ **User-Specific Rooms:**
- Each user has private room
- Jobs emitted only to job owner
- No cross-user data leakage

### Best Practices

✅ **HTTPS/WSS in Production:**
```nginx
location /socket.io/ {
    proxy_pass http://socket_server;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    # ... SSL enabled
}
```

✅ **CORS Configuration:**
```python
CORS(app, origins=settings.ALLOWED_ORIGINS_LIST, supports_credentials=True)

socketio = SocketIO(
    app,
    cors_allowed_origins=settings.ALLOWED_ORIGINS_LIST,
    async_mode='threading'
)
```

---

## Performance

### Connection Pooling

- Socket.IO uses a connection pool
- Multiple tabs from same user = multiple sockets
- All sockets in same room receive events

### Heartbeat

```python
@socketio.on('ping')
def handle_ping():
    emit('pong', {'timestamp': datetime.utcnow().isoformat()})
```

**Client Heartbeat:**
```typescript
setInterval(() => {
  socket.emit('ping')
}, 30000)  // Every 30 seconds
```

### Timeouts

```nginx
# Nginx WebSocket proxy timeouts
proxy_connect_timeout 7d;
proxy_send_timeout 7d;
proxy_read_timeout 7d;
```

---

## Debugging

### Server Logs

```python
logger.info(f"User connected: {username} (ID: {user_id}) - Session: {session_id}")
logger.info(f"Notified user {user_id} about job {job_id} start")
```

### Client Debugging

```typescript
// Enable debug mode
localStorage.setItem('debug', 'socket.io-client:*')

// Monitor events
socket.onAny((event, ...args) => {
  console.log(`Event: ${event}`, args)
})
```

### Test Connection

```bash
# Test socket health endpoint
curl http://localhost:5000/health

# Expected response:
{
  "status": "healthy",
  "service": "socket_server",
  "connected_users": 3
}
```

---

## Example: Complete Job Flow

```typescript
// 1. Upload files
const response = await uploadProtein(formData)
const jobId = response.job_id

// 2. Listen for events
socket.on('job_started', (data) => {
  if (data.job_id === jobId) {
    console.log('Processing started!')
  }
})

socket.on('job_progress', (data) => {
  if (data.job_id === jobId) {
    updateProgressBar(data.progress)
    console.log(data.message)
  }
})

socket.on('job_completed', (data) => {
  if (data.job_id === jobId) {
    showSuccessNotification('Job completed!')
    downloadFiles(data.output_files)
  }
})

socket.on('job_failed', (data) => {
  if (data.job_id === jobId) {
    showErrorNotification(data.error)
  }
})
```

---

## Troubleshooting

### Issue: Connection Refused

**Cause:** Socket server not running

**Solution:**
```bash
docker-compose ps  # Check socket service
docker-compose logs socket  # View logs
```

### Issue: 401 Authentication Error

**Cause:** Invalid or expired token

**Solution:**
```typescript
// Refresh token and reconnect
const newToken = await refreshAccessToken()
disconnectSocket()
connectSocket(newToken)
```

### Issue: Events Not Received

**Cause:** Not in correct room or wrong user_id

**Debug:**
```python
# Server-side logging
logger.info(f"Emitting to room: user_{user_id}")
logger.info(f"Connected users: {connected_users}")
```

---

## References

- [Socket.IO Documentation](https://socket.io/docs/v4/)
- [Flask-SocketIO Documentation](https://flask-socketio.readthedocs.io/)
- [socket.io-client (React)](https://socket.io/docs/v4/client-api/)
