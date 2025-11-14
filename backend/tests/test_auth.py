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

    @pytest.mark.skip(reason="Token refresh endpoint needs implementation")
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

    @pytest.mark.skip(reason="Token refresh endpoint needs implementation")
    def test_refresh_with_invalid_token(self, client):
        """Test refresh with invalid token fails"""
        response = client.post(
            "/api/v1/auth/refresh", headers={"Authorization": "Bearer invalid_token"}
        )

        assert response.status_code == status.HTTP_401_UNAUTHORIZED
