# Use an official Python runtime as a parent image
FROM python:3.11.4

# Set the working directory in the container to /app
WORKDIR /app

# Add the current directory contents into the container at /app
ADD . /app

# Install any needed packages specified in requirements.txt
RUN pip install --no-cache-dir -r requirements.txt

# Run setup.py
RUN python setup.py build_ext --inplace

# Make port 5000 available to the world outside this container
EXPOSE 5000

# Define environment variable
ENV NAME World

# Run uvicorn when the container launches
CMD ["uvicorn", "server:app", "--reload", "--host", "0.0.0.0", "--port", "5000"]
