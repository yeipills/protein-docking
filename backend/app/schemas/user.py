"""
Pydantic schemas for User operations
"""
from pydantic import BaseModel, EmailStr, Field, ConfigDict, field_validator
from typing import Optional
from datetime import datetime
import re


class UserBase(BaseModel):
    """Base user schema"""
    email: EmailStr
    username: str = Field(..., min_length=3, max_length=50)
    full_name: Optional[str] = None


class UserCreate(UserBase):
    """
    Schema for creating a new user

    Password requirements:
    - Minimum 12 characters
    - Maximum 72 characters (bcrypt limit)
    - At least one uppercase letter
    - At least one lowercase letter
    - At least one digit
    - At least one special character (!@#$%^&*(),.?\":{}|<>)
    """
    password: str = Field(..., min_length=12, max_length=72)

    @field_validator('password')
    @classmethod
    def validate_strong_password(cls, v: str) -> str:
        """
        Validate password strength

        Requirements:
        - Min 12 characters
        - At least 1 uppercase letter
        - At least 1 lowercase letter
        - At least 1 digit
        - At least 1 special character
        """
        if len(v) < 12:
            raise ValueError('Password must be at least 12 characters long')

        if not re.search(r'[A-Z]', v):
            raise ValueError('Password must contain at least one uppercase letter')

        if not re.search(r'[a-z]', v):
            raise ValueError('Password must contain at least one lowercase letter')

        if not re.search(r'\d', v):
            raise ValueError('Password must contain at least one digit')

        if not re.search(r'[!@#$%^&*(),.?\":{}|<>]', v):
            raise ValueError('Password must contain at least one special character (!@#$%^&*(),.?\":{}|<>)')

        # Check for common weak passwords
        weak_passwords = ['password123', 'Password123!', 'Qwerty123!', 'Admin123!']
        if v.lower() in [pwd.lower() for pwd in weak_passwords]:
            raise ValueError('Password is too common. Please choose a stronger password')

        return v


class UserLogin(BaseModel):
    """Schema for user login"""
    username: str
    password: str


class UserUpdate(BaseModel):
    """Schema for updating user information"""
    email: Optional[EmailStr] = None
    full_name: Optional[str] = None
    password: Optional[str] = Field(None, min_length=12, max_length=72)

    @field_validator('password')
    @classmethod
    def validate_strong_password(cls, v: Optional[str]) -> Optional[str]:
        """Validate password strength (same requirements as UserCreate)"""
        if v is None:
            return v

        # Apply same validation as UserCreate
        if len(v) < 12:
            raise ValueError('Password must be at least 12 characters long')

        if not re.search(r'[A-Z]', v):
            raise ValueError('Password must contain at least one uppercase letter')

        if not re.search(r'[a-z]', v):
            raise ValueError('Password must contain at least one lowercase letter')

        if not re.search(r'\d', v):
            raise ValueError('Password must contain at least one digit')

        if not re.search(r'[!@#$%^&*(),.?\":{}|<>]', v):
            raise ValueError('Password must contain at least one special character')

        weak_passwords = ['password123', 'Password123!', 'Qwerty123!', 'Admin123!']
        if v.lower() in [pwd.lower() for pwd in weak_passwords]:
            raise ValueError('Password is too common. Please choose a stronger password')

        return v


class UserResponse(UserBase):
    """Schema for user response"""
    id: int
    is_active: bool
    is_superuser: bool
    jobs_count: int
    created_at: datetime
    updated_at: Optional[datetime] = None

    model_config = ConfigDict(from_attributes=True)


class Token(BaseModel):
    """Schema for JWT token response"""
    access_token: str
    refresh_token: str
    token_type: str = "bearer"


class TokenData(BaseModel):
    """Schema for token payload data"""
    user_id: Optional[int] = None
    username: Optional[str] = None
