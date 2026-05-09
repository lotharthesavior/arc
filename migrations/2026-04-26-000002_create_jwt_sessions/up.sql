-- HIPAA-4 (§164.312(d)): server-side JWT registry. Token revocation requires
-- the framework to know which JWTs it issued and which have been invalidated.

CREATE TABLE jwt_sessions (
    jti TEXT NOT NULL PRIMARY KEY,
    actor_id TEXT NOT NULL,
    created_at_us BIGINT NOT NULL,
    expires_at_us BIGINT NOT NULL,
    revoked_at_us BIGINT
);

CREATE INDEX idx_jwt_sessions_actor_id ON jwt_sessions(actor_id);
CREATE INDEX idx_jwt_sessions_expires_at ON jwt_sessions(expires_at_us);
