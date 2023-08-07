# Use an official Python runtime as a parent image
FROM python:3.8

# Set the working directory in the container to /app/socket
WORKDIR /app/socket

# Add the current directory contents into the container at /app/socket
ADD . /app/socket

# Install any needed packages specified in requirements.txt
RUN pip install --no-cache-dir -r requirements.txt

# Set the FLASK_APP environment variable
ENV FLASK_APP=socketServer.py

# Make port 8000 available to the world outside this container
EXPOSE 8080

# Run the command to start the Flask app
CMD ["flask", "run", "--host=0.0.0.0", "--port=8080"]
