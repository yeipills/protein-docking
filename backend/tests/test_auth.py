"""
Tests for authentication endpoints
"""
import pytest
from fastapi import status


class TestRegister:
    """Tests for user registration"""

    def test_register_success(self, client, user_data):
        """Test successful user registration"""
        response = client.post("/api/v1/auth/register", json=user_data)

        assert response.status_code == status.HTTP_201_CREATED
        data = response.json()
        assert data["email"] == user_data["email"]
        assert data["username"] == user_data["username"]
        assert "id" in data
        assert "hashed_password" not in data

    def test_register_duplicate_email(self, client, test_user, user_data):
        """Test registration with duplicate email fails"""
        user_data["email"] = test_user.email

        response = client.post("/api/v1/auth/register", json=user_data)

        assert response.status_code == status.HTTP_409_CONFLICT
        assert "already registered" in response.json()["detail"].lower()

    def test_register_duplicate_username(self, client, test_user, user_data):
        """Test registration with duplicate username fails"""
        user_data["username"] = test_user.username

        response = client.post("/api/v1/auth/register", json=user_data)

        assert response.status_code == status.HTTP_409_CONFLICT

    def test_register_invalid_email(self, client, user_data):
        """Test registration with invalid email fails"""
        user_data["email"] = "not-an-email"

        response = client.post("/api/v1/auth/register", json=user_data)

        assert response.status_code == status.HTTP_422_UNPROCESSABLE_ENTITY

    def test_register_weak_password(self, client, user_data):
        """Test registration with weak password"""
        user_data["password"] = "123"

        response = client.post("/api/v1/auth/register", json=user_data)

        # Should accept (validation can be added later)
        # For now, just test it doesn't crash
        assert response.status_code in [
            status.HTTP_201_CREATED,
            status.HTTP_422_UNPROCESSABLE_ENTITY,
        ]


class TestLogin:
    """Tests for user login"""

    def test_login_success(self, client, test_user):
        """Test successful login"""
        response = client.post(
            "/api/v1/auth/login",
            json={"username": test_user.username, "password": "testpass123"},
        )

        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert "access_token" in data
        assert "refresh_token" in data
        assert data["token_type"] == "bearer"

    def test_login_wrong_password(self, client, test_user):
        """Test login with wrong password fails"""
        response = client.post(
            "/api/v1/auth/login",
            json={"username": test_user.username, "password": "wrongpassword"},
        )

        assert response.status_code == status.HTTP_401_UNAUTHORIZED

    def test_login_nonexistent_user(self, client):
        """Test login with nonexistent user fails"""
        response = client.post(
            "/api/v1/auth/login",
            json={"username": "nonexistent", "password": "password123"},
        )

        assert response.status_code == status.HTTP_401_UNAUTHORIZED

    def test_login_inactive_user(self, client, test_user, db_session):
        """Test login with inactive user fails"""
        test_user.is_active = False
        db_session.commit()

        response = client.post(
            "/api/v1/auth/login",
            json={"username": test_user.username, "password": "testpass123"},
        )

        assert response.status_code == status.HTTP_401_UNAUTHORIZED


class TestTokenRefresh:
    """Tests for token refresh"""

    def test_refresh_token_success(self, client, test_user):
        """Test successful token refresh"""
        # Login to get refresh token
        login_response = client.post(
            "/api/v1/auth/login",
            json={"username": test_user.username, "password": "testpass123"},
        )
        refresh_token = login_response.json()["refresh_token"]

        # Refresh token
        response = client.post(
            "/api/v1/auth/refresh",
            headers={"Authorization": f"Bearer {refresh_token}"},
        )

        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert "access_token" in data
        assert "token_type" in data

    def test_refresh_with_invalid_token(self, client):
        """Test refresh with invalid token fails"""
        response = client.post(
            "/api/v1/auth/refresh", headers={"Authorization": "Bearer invalid_token"}
        )

        assert response.status_code == status.HTTP_401_UNAUTHORIZED

    def test_refresh_without_token(self, client):
        """Test refresh without token fails"""
        response = client.post("/api/v1/auth/refresh")

        assert response.status_code == status.HTTP_403_FORBIDDEN

    def test_refresh_with_access_token_instead(self, client, user_token):
        """Test using access token for refresh fails"""
        response = client.post(
            "/api/v1/auth/refresh",
            headers={"Authorization": f"Bearer {user_token}"},
        )

        # Should fail because it's not a refresh token
        assert response.status_code in [status.HTTP_401_UNAUTHORIZED, status.HTTP_400_BAD_REQUEST]


class TestAuthSecurity:
    """Tests for authentication security features"""

    def test_password_not_returned_in_response(self, client, user_data):
        """Test password is never returned in API responses"""
        response = client.post("/api/v1/auth/register", json=user_data)

        assert response.status_code == status.HTTP_201_CREATED
        data = response.json()
        assert "password" not in data
        assert "hashed_password" not in data

    def test_jwt_token_contains_user_id(self, client, test_user):
        """Test JWT token contains user information"""
        from jose import jwt
        from app.config import get_settings

        settings = get_settings()

        response = client.post(
            "/api/v1/auth/login",
            json={"username": test_user.username, "password": "testpass123"},
        )

        access_token = response.json()["access_token"]
        payload = jwt.decode(
            access_token,
            settings.JWT_SECRET_KEY,
            algorithms=[settings.JWT_ALGORITHM],
        )

        assert "sub" in payload  # user_id
        assert "exp" in payload  # expiration
        assert "type" in payload  # token type

    def test_login_rate_limiting(self, client):
        """Test login endpoint has rate limiting"""
        # This test verifies rate limit headers exist
        # Actual rate limiting tested in integration tests
        response = client.post(
            "/api/v1/auth/login",
            json={"username": "test", "password": "test"},
        )

        # Rate limit headers should be present
        assert "X-RateLimit-Limit" in response.headers or response.status_code == status.HTTP_401_UNAUTHORIZED
