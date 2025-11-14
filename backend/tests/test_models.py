"""
Tests for User model
"""
import pytest
from app.models.user import User
from app.core.security import verify_password, get_password_hash


class TestUserModel:
    """Tests for User model"""

    def test_create_user(self, db_session):
        """Test creating a user"""
        user = User(
            email="test@example.com",
            username="testuser",
            full_name="Test User",
            hashed_password=get_password_hash("password123"),
        )
        db_session.add(user)
        db_session.commit()
        db_session.refresh(user)

        assert user.id is not None
        assert user.email == "test@example.com"
        assert user.username == "testuser"
        assert user.is_active is True
        assert user.is_superuser is False
        assert user.created_at is not None

    def test_user_password_hashing(self, db_session):
        """Test password is properly hashed"""
        password = "securepassword123"
        user = User(
            email="test@example.com",
            username="testuser",
            hashed_password=get_password_hash(password),
        )

        # Password should be hashed
        assert user.hashed_password != password

        # Should verify correctly
        assert verify_password(password, user.hashed_password)

        # Wrong password should not verify
        assert not verify_password("wrongpassword", user.hashed_password)

    def test_user_unique_email(self, db_session, test_user):
        """Test email must be unique"""
        duplicate_user = User(
            email=test_user.email,  # Same email
            username="different",
            hashed_password=get_password_hash("password"),
        )
        db_session.add(duplicate_user)

        with pytest.raises(Exception):  # Should raise IntegrityError
            db_session.commit()

    def test_user_unique_username(self, db_session, test_user):
        """Test username must be unique"""
        duplicate_user = User(
            email="different@example.com",
            username=test_user.username,  # Same username
            hashed_password=get_password_hash("password"),
        )
        db_session.add(duplicate_user)

        with pytest.raises(Exception):  # Should raise IntegrityError
            db_session.commit()

    def test_user_repr(self, test_user):
        """Test user string representation"""
        repr_str = repr(test_user)
        assert "User" in repr_str
        assert str(test_user.id) in repr_str
        assert test_user.email in repr_str
        assert test_user.username in repr_str


class TestProteinModel:
    """Tests for Protein model"""

    def test_create_protein(self, db_session, test_user):
        """Test creating a protein"""
        from app.models.protein import Protein

        protein = Protein(
            user_id=test_user.id,
            name="Test Protein",
            description="A test protein",
            stl_file="/path/to/file.stl",
        )
        db_session.add(protein)
        db_session.commit()
        db_session.refresh(protein)

        assert protein.id is not None
        assert protein.user_id == test_user.id
        assert protein.name == "Test Protein"
        assert protein.is_public is False
        assert protein.is_deleted is False
        assert protein.created_at is not None

    def test_protein_relationship_with_user(self, db_session, test_protein):
        """Test protein belongs to user"""
        assert test_protein.user is not None
        assert test_protein.user.id == test_protein.user_id

    def test_protein_soft_delete(self, db_session, test_protein):
        """Test soft delete functionality"""
        test_protein.is_deleted = True
        db_session.commit()

        assert test_protein.is_deleted is True

    def test_protein_metadata_json(self, db_session, test_user):
        """Test JSONB fields for metadata"""
        from app.models.protein import Protein

        metadata = {
            "centroid_count": 1000,
            "processing_time": 120,
            "algorithm_version": "2.1.0",
        }

        protein = Protein(
            user_id=test_user.id,
            name="Test Protein",
            processing_metadata=metadata,
        )
        db_session.add(protein)
        db_session.commit()
        db_session.refresh(protein)

        assert protein.processing_metadata == metadata
        assert protein.processing_metadata["centroid_count"] == 1000


class TestJobModel:
    """Tests for Job model"""

    def test_create_job(self, db_session, test_user, test_protein):
        """Test creating a job"""
        from app.models.job import Job, JobType, JobStatus

        job = Job(
            user_id=test_user.id,
            protein_id=test_protein.id,
            job_type=JobType.PART_ONE,
            status=JobStatus.PENDING,
        )
        db_session.add(job)
        db_session.commit()
        db_session.refresh(job)

        assert job.id is not None
        assert job.user_id == test_user.id
        assert job.protein_id == test_protein.id
        assert job.job_type == JobType.PART_ONE
        assert job.status == JobStatus.PENDING
        assert job.progress == 0

    def test_job_status_transition(self, db_session, test_job):
        """Test job status transitions"""
        from app.models.job import JobStatus
        from datetime import datetime

        # Pending -> Processing
        test_job.status = JobStatus.PROCESSING
        test_job.started_at = datetime.utcnow()
        db_session.commit()

        assert test_job.status == JobStatus.PROCESSING
        assert test_job.started_at is not None

        # Processing -> Completed
        test_job.status = JobStatus.COMPLETED
        test_job.completed_at = datetime.utcnow()
        test_job.progress = 100
        db_session.commit()

        assert test_job.status == JobStatus.COMPLETED
        assert test_job.completed_at is not None
        assert test_job.progress == 100

    def test_job_relationships(self, db_session, test_job):
        """Test job relationships with user and protein"""
        assert test_job.user is not None
        assert test_job.user.id == test_job.user_id
        assert test_job.protein is not None
        assert test_job.protein.id == test_job.protein_id

    def test_job_output_files_json(self, db_session, test_job):
        """Test JSONB fields for output files"""
        output_files = {
            "cr_totals": "/path/to/cr_totals.txt",
            "context_rays": "/path/to/context_rays.txt",
        }

        test_job.output_files = output_files
        db_session.commit()
        db_session.refresh(test_job)

        assert test_job.output_files == output_files
        assert test_job.output_files["cr_totals"] == "/path/to/cr_totals.txt"

    def test_job_without_protein(self, db_session, test_user):
        """Test job can be created without protein_id"""
        from app.models.job import Job, JobType, JobStatus

        job = Job(
            user_id=test_user.id,
            protein_id=None,  # Optional
            job_type=JobType.PART_TWO,
            status=JobStatus.PENDING,
        )
        db_session.add(job)
        db_session.commit()

        assert job.protein_id is None
        assert job.protein is None


class TestModelRelationships:
    """Tests for model relationships and cascades"""

    def test_user_has_proteins(self, db_session, test_user, test_protein):
        """Test user.proteins relationship"""
        assert len(test_user.proteins) > 0
        assert test_protein in test_user.proteins

    def test_user_has_jobs(self, db_session, test_user, test_job):
        """Test user.jobs relationship"""
        assert len(test_user.jobs) > 0
        assert test_job in test_user.jobs

    def test_protein_has_jobs(self, db_session, test_protein, test_job):
        """Test protein.jobs relationship"""
        assert len(test_protein.jobs) > 0
        assert test_job in test_protein.jobs

    def test_cascade_delete_user_deletes_proteins(self, db_session, test_user):
        """Test deleting user cascades to proteins"""
        from app.models.protein import Protein

        # Create protein
        protein = Protein(
            user_id=test_user.id, name="Test Protein", description="Test"
        )
        db_session.add(protein)
        db_session.commit()
        protein_id = protein.id

        # Delete user
        db_session.delete(test_user)
        db_session.commit()

        # Protein should be deleted
        deleted_protein = db_session.query(Protein).filter_by(id=protein_id).first()
        assert deleted_protein is None
