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
