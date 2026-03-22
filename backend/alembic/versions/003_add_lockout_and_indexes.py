"""Add account lockout fields and database indexes

Revision ID: 003
Revises: 002
Create Date: 2025-11-15

Changes:
- Add failed_login_attempts, last_failed_login, locked_until to users table
- Add composite indexes for better query performance
- Add indexes on created_at and status fields for jobs

"""
from alembic import op
import sqlalchemy as sa

# revision identifiers, used by Alembic.
revision = '003'
down_revision = '002'
branch_labels = None
depends_on = None


def upgrade() -> None:
    """Add account lockout fields and performance indexes"""

    # Add account lockout fields to users table
    op.add_column('users', sa.Column('failed_login_attempts', sa.Integer(), nullable=True, server_default='0'))
    op.add_column('users', sa.Column('last_failed_login', sa.DateTime(timezone=True), nullable=True))
    op.add_column('users', sa.Column('locked_until', sa.DateTime(timezone=True), nullable=True))

    # Create composite indexes for better query performance
    # NOTE: ix_jobs_user_created already exists in migration 001 as idx_jobs_user_created
    # NOTE: ix_proteins_user_created already exists in migration 001 as idx_proteins_user_created
    # NOTE: ix_jobs_status already exists in migration 001

    # Index for queries filtering by user_id, status and created_at (NEW)
    op.create_index(
        'ix_jobs_user_status_created',
        'jobs',
        ['user_id', 'status', 'created_at'],
        unique=False
    )

    # Index on created_at for time-based queries (NEW)
    op.create_index(
        'ix_jobs_created_at',
        'jobs',
        ['created_at'],
        unique=False
    )


def downgrade() -> None:
    """Remove account lockout fields and indexes"""

    # Drop only the indexes we created in this migration
    op.drop_index('ix_jobs_created_at', table_name='jobs')
    op.drop_index('ix_jobs_user_status_created', table_name='jobs')

    # Drop lockout columns
    op.drop_column('users', 'locked_until')
    op.drop_column('users', 'last_failed_login')
    op.drop_column('users', 'failed_login_attempts')
