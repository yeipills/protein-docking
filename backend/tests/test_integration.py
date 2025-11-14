"""
Integration tests for critical workflows
"""
import pytest
from fastapi import status


@pytest.mark.integration
class TestUserRegistrationAndLogin:
    """Test complete user registration and login flow"""

    def test_register_login_workflow(self, client):
        """Test user can register and then login"""
        # Register
        register_data = {
            "email": "integration@test.com",
            "username": "integrationuser",
            "full_name": "Integration Test",
            "password": "securepass123",
        }
        register_response = client.post("/api/v1/auth/register", json=register_data)
        assert register_response.status_code == status.HTTP_201_CREATED

        # Login with same credentials
        login_response = client.post(
            "/api/v1/auth/login",
            json={"username": "integrationuser", "password": "securepass123"},
        )
        assert login_response.status_code == status.HTTP_200_OK
        assert "access_token" in login_response.json()

        # Access protected endpoint
        token = login_response.json()["access_token"]
        me_response = client.get(
            "/api/v1/users/me", headers={"Authorization": f"Bearer {token}"}
        )
        assert me_response.status_code == status.HTTP_200_OK
        assert me_response.json()["username"] == "integrationuser"


@pytest.mark.integration
class TestJobLifecycle:
    """Test complete job lifecycle"""

    def test_job_creation_to_cancellation(
        self, client, auth_headers, test_user, db_session
    ):
        """Test job from creation through cancellation"""
        from app.models.job import Job, JobType, JobStatus

        # Create job
        job = Job(
            user_id=test_user.id,
            job_type=JobType.PART_ONE,
            status=JobStatus.PENDING,
        )
        db_session.add(job)
        db_session.commit()
        db_session.refresh(job)

        # Get job details
        get_response = client.get(f"/api/v1/jobs/{job.id}", headers=auth_headers)
        assert get_response.status_code == status.HTTP_200_OK

        # Cancel job
        cancel_response = client.post(
            f"/api/v1/jobs/{job.id}/cancel", headers=auth_headers
        )
        assert cancel_response.status_code == status.HTTP_200_OK
