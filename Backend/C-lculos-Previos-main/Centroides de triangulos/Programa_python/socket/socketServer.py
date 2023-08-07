from http.client import HTTPException
from flask import Flask, request, jsonify, render_template
from flask_socketio import SocketIO, emit
from flask_cors import CORS
app = Flask(__name__)
app.config['SECRET_KEY'] = ''
CORS(app, resources={r"/*": {"origins": "*"}})

socketio = SocketIO(app, cors_allowed_origins="*")
thread = None


@app.get("/")
def read_root():
    return {"Backend": "Proyecto titulo"}


@app.get("/terminaParteUno")
def termina_parte_uno():
    socketio.emit("termina_procesar_uno")
    return jsonify(message="termina_procesar_uno emitted"), 200


@app.get("/terminaParteDos")
def termina_parte_dos():
    socketio.emit("termina_procesar_dos")
    return jsonify(message="termina_procesar_dos emitted"), 200

@socketio.on("connect")
def connected():
    """event listener when client connects to the server"""
    print(request.sid)
    print("client has connected")
    emit("connected_user", request.sid)


@socketio.on("disconnect")
def disconnected():
    print("user disconnected")
    emit("disconnect")


@app.errorhandler(Exception)
def handle_exception(e):
    # pass through HTTP errors
    if isinstance(e, HTTPException):
        return e

    # now you're handling non-HTTP exceptions only
    return render_template("500_generic.html", e=e), 500


if __name__ == '__main__':
    socketio.run(app, debug=True, port=5000)