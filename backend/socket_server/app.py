"""
Socket.IO server for real-time notifications
Supports user authentication and individual rooms
"""
from flask import Flask
from flask_socketio import SocketIO, emit, join_room, leave_room
from flask_cors import CORS
import os
from datetime import datetime
from app.config import get_settings
from app.core.security import decode_token
from app.core.logging import setup_logging, get_logger

# Setup logging
setup_logging()
logger = get_logger(__name__)

# Load settings
settings = get_settings()

# Create Flask app
app = Flask(__name__)
app.config['SECRET_KEY'] = settings.SOCKET_SECRET_KEY

# Enable CORS
CORS(app, origins=settings.ALLOWED_ORIGINS_LIST, supports_credentials=True)

# Create SocketIO instance
socketio = SocketIO(
    app,
    cors_allowed_origins=settings.ALLOWED_ORIGINS_LIST,
    async_mode='threading',
    logger=True,
    engineio_logger=False
)

# Store connected users
connected_users = {}


@socketio.on('connect')
def handle_connect(auth):
    """Handle client connection with authentication"""
    try:
        # Get token from auth
        if not auth or 'token' not in auth:
            logger.warning("Connection attempt without token")
            return False

        token = auth['token']

        # Verify token
        payload = decode_token(token)
        if not payload:
            logger.warning("Connection attempt with invalid token")
            return False

        user_id = payload.get('user_id')
        username = payload.get('username')

        if not user_id:
            logger.warning("Token missing user_id")
            return False

        # Store connection
        from flask import request
        session_id = request.sid
        connected_users[session_id] = {
            'user_id': user_id,
            'username': username
        }

        # Join user-specific room
        room = f"user_{user_id}"
        join_room(room)

        logger.info(f"User connected: {username} (ID: {user_id}) - Session: {session_id}")

        # Send welcome message
        emit('connected', {
            'message': 'Successfully connected to Protein Docking Platform',
            'user_id': user_id,
            'username': username
        })

        return True

    except Exception as e:
        logger.error(f"Error during connection: {str(e)}", exc_info=True)
        return False


@socketio.on('disconnect')
def handle_disconnect():
    """Handle client disconnection"""
    try:
        from flask import request
        session_id = request.sid

        if session_id in connected_users:
            user_info = connected_users[session_id]
            user_id = user_info['user_id']
            username = user_info['username']

            # Leave room
            room = f"user_{user_id}"
            leave_room(room)

            # Remove from connected users
            del connected_users[session_id]

            logger.info(f"User disconnected: {username} (ID: {user_id})")

    except Exception as e:
        logger.error(f"Error during disconnection: {str(e)}", exc_info=True)


@socketio.on('ping')
def handle_ping():
    """Handle ping from client"""
    emit('pong', {'timestamp': datetime.utcnow().isoformat()})


def notify_job_started(user_id: int, job_id: int, job_type: str):
    """
    Notify user that a job has started

    Args:
        user_id: User ID
        job_id: Job ID
        job_type: Type of job (part_one or part_two)
    """
    room = f"user_{user_id}"
    socketio.emit('job_started', {
        'job_id': job_id,
        'job_type': job_type,
        'message': f'Processing started for {job_type}'
    }, room=room)
    logger.info(f"Notified user {user_id} about job {job_id} start")


def notify_job_progress(user_id: int, job_id: int, progress: int, message: str = None):
    """
    Notify user about job progress

    Args:
        user_id: User ID
        job_id: Job ID
        progress: Progress percentage (0-100)
        message: Optional progress message
    """
    room = f"user_{user_id}"
    socketio.emit('job_progress', {
        'job_id': job_id,
        'progress': progress,
        'message': message
    }, room=room)


def notify_job_completed(user_id: int, job_id: int, job_type: str, output_files: list = None):
    """
    Notify user that a job has completed

    Args:
        user_id: User ID
        job_id: Job ID
        job_type: Type of job (part_one or part_two)
        output_files: List of output file paths
    """
    room = f"user_{user_id}"
    socketio.emit('job_completed', {
        'job_id': job_id,
        'job_type': job_type,
        'message': f'Processing completed for {job_type}',
        'output_files': output_files or []
    }, room=room)
    logger.info(f"Notified user {user_id} about job {job_id} completion")


def notify_job_failed(user_id: int, job_id: int, error_message: str):
    """
    Notify user that a job has failed

    Args:
        user_id: User ID
        job_id: Job ID
        error_message: Error message
    """
    room = f"user_{user_id}"
    socketio.emit('job_failed', {
        'job_id': job_id,
        'error': error_message
    }, room=room)
    logger.info(f"Notified user {user_id} about job {job_id} failure")


@app.route('/health')
def health():
    """Health check endpoint"""
    return {
        'status': 'healthy',
        'service': 'socket_server',
        'connected_users': len(connected_users)
    }


if __name__ == '__main__':
    logger.info(f"Starting Socket.IO server on {settings.SOCKET_HOST}:{settings.SOCKET_PORT}")
    socketio.run(
        app,
        host=settings.SOCKET_HOST,
        port=settings.SOCKET_PORT,
        debug=settings.ENVIRONMENT == 'development',
        allow_unsafe_werkzeug=True  # Required for development mode
    )
