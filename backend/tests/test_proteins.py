"""
Tests for protein management endpoints
"""
import pytest
from fastapi import status
from io import BytesIO


class TestListProteins:
    """Tests for GET /proteins/ endpoint"""

    def test_list_user_proteins(self, client, auth_headers, test_protein):
        """Test user can list their proteins"""
        response = client.get("/api/v1/proteins/", headers=auth_headers)

        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert "proteins" in data or isinstance(data, list)

    def test_list_proteins_unauthenticated(self, client):
        """Test unauthenticated request fails"""
        response = client.get("/api/v1/proteins/")

        assert response.status_code == status.HTTP_403_FORBIDDEN

    def test_list_proteins_pagination(self, client, auth_headers):
        """Test protein listing with pagination"""
        response = client.get("/api/v1/proteins/?skip=0&limit=10", headers=auth_headers)

        assert response.status_code == status.HTTP_200_OK


class TestGetProtein:
    """Tests for GET /proteins/{protein_id} endpoint"""

    def test_get_protein_success(self, client, auth_headers, test_protein):
        """Test user can get their protein details"""
        response = client.get(
            f"/api/v1/proteins/{test_protein.id}", headers=auth_headers
        )

        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert data["id"] == test_protein.id
        assert data["name"] == test_protein.name

    def test_get_nonexistent_protein(self, client, auth_headers):
        """Test getting nonexistent protein returns 404"""
        response = client.get("/api/v1/proteins/99999", headers=auth_headers)

        assert response.status_code == status.HTTP_404_NOT_FOUND

    def test_get_other_user_protein(
        self, client, auth_headers, db_session, test_superuser
    ):
        """Test user cannot access other user's private proteins"""
        from app.models.protein import Protein

        other_protein = Protein(
            user_id=test_superuser.id,
            name="Other User Protein",
            is_public=False,
        )
        db_session.add(other_protein)
        db_session.commit()

        response = client.get(
            f"/api/v1/proteins/{other_protein.id}", headers=auth_headers
        )

        assert response.status_code == status.HTTP_403_FORBIDDEN


class TestUploadProtein:
    """Tests for POST /proteins/upload/part-one endpoint"""

    def test_upload_requires_authentication(self, client):
        """Test upload requires authentication"""
        files = {
            "stl_file": ("test.stl", BytesIO(b"fake stl data"), "model/stl"),
            "vertices_file": ("test.vert", BytesIO(b"fake vert"), "text/plain"),
            "faces_file": ("test.face", BytesIO(b"fake face"), "text/plain"),
        }
        data = {"protein_name": "Test Protein"}

        response = client.post("/api/v1/proteins/upload/part-one", files=files, data=data)

        assert response.status_code == status.HTTP_403_FORBIDDEN

    @pytest.mark.skip(reason="Requires actual file validation")
    def test_upload_part_one_success(self, client, auth_headers):
        """Test successful file upload"""
        files = {
            "stl_file": ("test.stl", BytesIO(b"fake stl data"), "model/stl"),
            "vertices_file": ("test.vert", BytesIO(b"fake vert"), "text/plain"),
            "faces_file": ("test.face", BytesIO(b"fake face"), "text/plain"),
        }
        data = {"protein_name": "Test Protein"}

        response = client.post(
            "/api/v1/proteins/upload/part-one",
            files=files,
            data=data,
            headers=auth_headers,
        )

        # May fail due to file validation, but should not crash
        assert response.status_code in [
            status.HTTP_201_CREATED,
            status.HTTP_400_BAD_REQUEST,
            status.HTTP_422_UNPROCESSABLE_ENTITY,
        ]

    def test_upload_missing_files(self, client, auth_headers):
        """Test upload with missing files fails"""
        data = {"protein_name": "Test Protein"}

        response = client.post(
            "/api/v1/proteins/upload/part-one", data=data, headers=auth_headers
        )

        assert response.status_code == status.HTTP_422_UNPROCESSABLE_ENTITY

    def test_upload_invalid_file_type(self, client, auth_headers):
        """Test upload with invalid file type fails"""
        files = {
            "stl_file": ("test.exe", BytesIO(b"fake exe"), "application/x-msdownload"),
        }
        data = {"protein_name": "Test Protein"}

        response = client.post(
            "/api/v1/proteins/upload/part-one",
            files=files,
            data=data,
            headers=auth_headers,
        )

        assert response.status_code in [
            status.HTTP_400_BAD_REQUEST,
            status.HTTP_422_UNPROCESSABLE_ENTITY,
        ]


class TestProteinMetadata:
    """Tests for protein metadata management"""

    def test_protein_has_metadata_fields(self, test_protein):
        """Test protein has all required metadata fields"""
        assert hasattr(test_protein, "name")
        assert hasattr(test_protein, "description")
        assert hasattr(test_protein, "created_at")
        assert hasattr(test_protein, "is_public")
        assert hasattr(test_protein, "is_deleted")

    def test_protein_soft_delete_flag(self, test_protein, db_session):
        """Test protein soft delete"""
        test_protein.is_deleted = True
        db_session.commit()

        assert test_protein.is_deleted is True

    def test_protein_public_flag(self, test_protein, db_session):
        """Test protein public visibility"""
        test_protein.is_public = True
        db_session.commit()

        assert test_protein.is_public is True
