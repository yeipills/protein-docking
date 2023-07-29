import json
from flask import Flask, request, jsonify, current_app, abort
from flask_restful import Api, Resource
from flask_socketio import SocketIO, emit
from html.parser import HTMLParser
import logging
import bleach

app = Flask(__name__)
api = Api(app)
socketio = SocketIO(app, cors_allowed_origins="*")


class HelloWorldResource(Resource):
    def get(self):
        try:
            return jsonify({'hello': 'world'}), 200
        except ValueError as e:
            current_app.logger.error(str(e))
            abort(400, str(e))
        except KeyError as e:
            current_app.logger.error(str(e))
            abort(404, str(e))
        except Exception as e:
            current_app.logger.error(str(e))
            abort(500, str(e))


@app.errorhandler(400)
def handle_bad_request(error):
    response = jsonify({'error': 'Bad Request'})
    response.status_code = 400
    return response


@app.errorhandler(404)
def handle_not_found(error):
    response = jsonify({'error': 'Not Found'})
    response.status_code = 404
    return response


@app.errorhandler(500)
def handle_internal_server_error(error):
    response = jsonify({'error': 'Internal Server Error'})
    response.status_code = 500
    return response


api.add_resource(HelloWorldResource, '/')

logger = logging.getLogger(__name__)


def validate_data(data):
    if data is None:
        return False
    if not isinstance(data, str):
        raise TypeError(f'Invalid data type: {type(data)}')

    cleaned_data = bleach.clean(data)

    return True


def handle_message(data):
    logger.info('received message: %s', data)

    if not validate_data(data):
        return

    try:
        pass
    except Exception as e:
        logger.error('An error occurred: %s', str(e))
        socketio.emit('response', f'Error: {str(e)}')
        return

    response = f'Message received: {data}'
    socketio.emit('response', response)


@socketio.on('connect')
def handle_connect():
    logger.debug('Client connected')
    try:
        emit('connection_success', {'message': 'Connected successfully'})
    except Exception as e:
        logger.error(f'Error during connection: {e}')
        emit('connection_error', {'message': str(e)})


@socketio.on('disconnect')
def handle_disconnect():
    logger.debug('Client disconnected')


if __name__ == '__main__':
    socketio.run(app, host='localhost', debug=True, port=8000)
