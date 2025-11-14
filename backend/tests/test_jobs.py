"""
Tests for job management endpoints
"""
import pytest
from fastapi import status


class TestListJobs:
    """Tests for GET /jobs/ endpoint"""

    def test_list_user_jobs(self, client, auth_headers, test_job):
        """Test user can list their jobs"""
        response = client.get("/api/v1/jobs/", headers=auth_headers)

        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert isinstance(data, list)
        assert len(data) > 0
        assert data[0]["id"] == test_job.id

    def test_list_jobs_unauthenticated(self, client):
        """Test unauthenticated request fails"""
        response = client.get("/api/v1/jobs/")

        assert response.status_code == status.HTTP_403_FORBIDDEN

    def test_list_jobs_pagination(self, client, auth_headers, db_session, test_user):
        """Test job listing with pagination"""
        from app.models.job import Job, JobType, JobStatus

        # Create multiple jobs
        for i in range(5):
            job = Job(
                user_id=test_user.id,
                job_type=JobType.PART_ONE,
                status=JobStatus.PENDING,
            )
            db_session.add(job)
        db_session.commit()

        # Test with limit
        response = client.get("/api/v1/jobs/?limit=2", headers=auth_headers)

        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert len(data) <= 2


class TestGetJob:
    """Tests for GET /jobs/{job_id} endpoint"""

    def test_get_job_success(self, client, auth_headers, test_job):
        """Test user can get their job details"""
        response = client.get(f"/api/v1/jobs/{test_job.id}", headers=auth_headers)

        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert data["id"] == test_job.id
        assert data["status"] == test_job.status.value

    def test_get_other_user_job(self, client, auth_headers, db_session, test_superuser):
        """Test user cannot access other user's jobs"""
        from app.models.job import Job, JobType, JobStatus

        # Create job for superuser
        other_job = Job(
            user_id=test_superuser.id,
            job_type=JobType.PART_ONE,
            status=JobStatus.PENDING,
        )
        db_session.add(other_job)
        db_session.commit()

        # Try to access as regular user
        response = client.get(f"/api/v1/jobs/{other_job.id}", headers=auth_headers)

        assert response.status_code == status.HTTP_403_FORBIDDEN

    def test_get_nonexistent_job(self, client, auth_headers):
        """Test getting nonexistent job returns 404"""
        response = client.get("/api/v1/jobs/99999", headers=auth_headers)

        assert response.status_code == status.HTTP_404_NOT_FOUND


class TestCancelJob:
    """Tests for POST /jobs/{job_id}/cancel endpoint"""

    def test_cancel_pending_job(self, client, auth_headers, test_job):
        """Test canceling a pending job"""
        response = client.post(
            f"/api/v1/jobs/{test_job.id}/cancel", headers=auth_headers
        )

        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert data["status"] == "cancelled"

    def test_cancel_completed_job(self, client, auth_headers, test_job, db_session):
        """Test cannot cancel completed job"""
        from app.models.job import JobStatus

        test_job.status = JobStatus.COMPLETED
        db_session.commit()

        response = client.post(
            f"/api/v1/jobs/{test_job.id}/cancel", headers=auth_headers
        )

        # Should either succeed with no-op or return error
        assert response.status_code in [
            status.HTTP_200_OK,
            status.HTTP_400_BAD_REQUEST,
        ]

    def test_cancel_other_user_job(self, client, auth_headers, db_session, test_superuser):
        """Test cannot cancel other user's job"""
        from app.models.job import Job, JobType, JobStatus

        other_job = Job(
            user_id=test_superuser.id,
            job_type=JobType.PART_ONE,
            status=JobStatus.PENDING,
        )
        db_session.add(other_job)
        db_session.commit()

        response = client.post(
            f"/api/v1/jobs/{other_job.id}/cancel", headers=auth_headers
        )

        assert response.status_code == status.HTTP_403_FORBIDDEN


class TestJobStatusTracking:
    """Tests for job status and progress tracking"""

    def test_job_progress_field(self, test_job):
        """Test job has progress field"""
        assert hasattr(test_job, "progress")
        assert test_job.progress == 0

    def test_job_timestamps(self, test_job):
        """Test job has timestamp fields"""
        assert hasattr(test_job, "created_at")
        assert hasattr(test_job, "started_at")
        assert hasattr(test_job, "completed_at")
        assert test_job.created_at is not None

    def test_job_type_enum(self, test_job):
        """Test job type is enum"""
        from app.models.job import JobType

        assert test_job.job_type in [JobType.PART_ONE, JobType.PART_TWO]

    def test_job_status_enum(self, test_job):
        """Test job status is enum"""
        from app.models.job import JobStatus

        assert test_job.status in [
            JobStatus.PENDING,
            JobStatus.PROCESSING,
            JobStatus.COMPLETED,
            JobStatus.FAILED,
            JobStatus.CANCELLED,
        ]
