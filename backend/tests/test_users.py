"""
Tests for user management endpoints
"""
import pytest
from fastapi import status


class TestGetCurrentUser:
    """Tests for GET /users/me endpoint"""

    def test_get_current_user_success(self, client, auth_headers, test_user):
        """Test authenticated user can get their profile"""
        response = client.get("/api/v1/users/me", headers=auth_headers)

        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert data["id"] == test_user.id
        assert data["email"] == test_user.email
        assert data["username"] == test_user.username
        assert "hashed_password" not in data

    def test_get_current_user_unauthenticated(self, client):
        """Test unauthenticated request fails"""
        response = client.get("/api/v1/users/me")

        assert response.status_code == status.HTTP_403_FORBIDDEN

    def test_get_current_user_invalid_token(self, client):
        """Test invalid token fails"""
        response = client.get(
            "/api/v1/users/me", headers={"Authorization": "Bearer invalid_token"}
        )

        assert response.status_code == status.HTTP_401_UNAUTHORIZED


class TestUpdateCurrentUser:
    """Tests for PUT /users/me endpoint"""

    def test_update_user_profile(self, client, auth_headers, test_user, db_session):
        """Test user can update their profile"""
        update_data = {
            "full_name": "Updated Name",
            "email": "updated@example.com",
        }

        response = client.put(
            "/api/v1/users/me", headers=auth_headers, json=update_data
        )

        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert data["full_name"] == "Updated Name"
        assert data["email"] == "updated@example.com"

    def test_update_user_email_conflict(
        self, client, auth_headers, test_user, test_superuser
    ):
        """Test updating email to existing email fails"""
        update_data = {"email": test_superuser.email}

        response = client.put(
            "/api/v1/users/me", headers=auth_headers, json=update_data
        )

        assert response.status_code in [
            status.HTTP_409_CONFLICT,
            status.HTTP_400_BAD_REQUEST,
        ]

    def test_update_user_invalid_email(self, client, auth_headers):
        """Test updating with invalid email fails"""
        update_data = {"email": "not-an-email"}

        response = client.put(
            "/api/v1/users/me", headers=auth_headers, json=update_data
        )

        assert response.status_code == status.HTTP_422_UNPROCESSABLE_ENTITY


class TestAdminUserManagement:
    """Tests for admin user management endpoints"""

    def test_admin_get_user_by_id(self, client, admin_headers, test_user):
        """Test admin can get any user by ID"""
        response = client.get(
            f"/api/v1/users/{test_user.id}", headers=admin_headers
        )

        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert data["id"] == test_user.id

    def test_non_admin_cannot_get_other_user(self, client, auth_headers, test_superuser):
        """Test regular user cannot get other users"""
        response = client.get(
            f"/api/v1/users/{test_superuser.id}", headers=auth_headers
        )

        assert response.status_code == status.HTTP_403_FORBIDDEN

    def test_admin_list_all_users(self, client, admin_headers, test_user):
        """Test admin can list all users"""
        response = client.get("/api/v1/users/", headers=admin_headers)

        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert isinstance(data, list)
        assert len(data) > 0

    def test_non_admin_cannot_list_users(self, client, auth_headers):
        """Test regular user cannot list all users"""
        response = client.get("/api/v1/users/", headers=auth_headers)

        assert response.status_code == status.HTTP_403_FORBIDDEN

    def test_get_nonexistent_user(self, client, admin_headers):
        """Test getting nonexistent user returns 404"""
        response = client.get("/api/v1/users/99999", headers=admin_headers)

        assert response.status_code == status.HTTP_404_NOT_FOUND
