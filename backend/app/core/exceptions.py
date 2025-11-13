"""
Custom exceptions for the Protein Docking Platform
"""
from fastapi import HTTPException, status


class ProteinDockingException(HTTPException):
    """Base exception for all custom exceptions"""

    def __init__(
        self,
        status_code: int,
        detail: str,
        headers: dict = None
    ):
        super().__init__(status_code=status_code, detail=detail, headers=headers)


class UnauthorizedException(ProteinDockingException):
    """Exception raised when authentication fails"""

    def __init__(self, detail: str = "Could not validate credentials"):
        super().__init__(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail=detail,
            headers={"WWW-Authenticate": "Bearer"}
        )


class NotFoundException(ProteinDockingException):
    """Exception raised when a resource is not found"""

    def __init__(self, detail: str = "Resource not found"):
        super().__init__(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=detail
        )


class ValidationException(ProteinDockingException):
    """Exception raised when validation fails"""

    def __init__(self, detail: str = "Validation error"):
        super().__init__(
            status_code=status.HTTP_422_UNPROCESSABLE_ENTITY,
            detail=detail
        )


class RateLimitException(ProteinDockingException):
    """Exception raised when rate limit is exceeded"""

    def __init__(self, detail: str = "Rate limit exceeded"):
        super().__init__(
            status_code=status.HTTP_429_TOO_MANY_REQUESTS,
            detail=detail
        )


class ForbiddenException(ProteinDockingException):
    """Exception raised when user doesn't have permission"""

    def __init__(self, detail: str = "Permission denied"):
        super().__init__(
            status_code=status.HTTP_403_FORBIDDEN,
            detail=detail
        )


class ConflictException(ProteinDockingException):
    """Exception raised when there's a conflict (e.g., duplicate resource)"""

    def __init__(self, detail: str = "Resource already exists"):
        super().__init__(
            status_code=status.HTTP_409_CONFLICT,
            detail=detail
        )


class InternalServerException(ProteinDockingException):
    """Exception raised for internal server errors"""

    def __init__(self, detail: str = "Internal server error"):
        super().__init__(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=detail
        )
