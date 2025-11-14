"""
Tests for health check and root endpoints
"""
from fastapi import status


def test_root_endpoint(client):
    """Test root endpoint returns correct information"""
    response = client.get("/")

    assert response.status_code == status.HTTP_200_OK
    data = response.json()
    assert data["message"] == "Protein Docking Platform API"
    assert "version" in data
    assert "docs" in data
    assert "health" in data


def test_health_check(client):
    """Test health check endpoint"""
    response = client.get("/health")

    assert response.status_code == status.HTTP_200_OK
    data = response.json()
    assert data["status"] == "healthy"
    assert "environment" in data
    assert "version" in data


def test_metrics_endpoint(client):
    """Test Prometheus metrics endpoint exists"""
    response = client.get("/metrics")

    # Metrics endpoint should return text/plain
    assert response.status_code == status.HTTP_200_OK
    assert "text/plain" in response.headers.get("content-type", "")
