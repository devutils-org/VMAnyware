-- Last guest address proven by readiness: a connection hint, never host identity.
ALTER TABLE instances ADD COLUMN guest_address TEXT CHECK (guest_address IS NULL OR length(guest_address) BETWEEN 7 AND 15);
ALTER TABLE instances ADD COLUMN guest_user TEXT CHECK (guest_user IS NULL OR length(guest_user) BETWEEN 1 AND 32);
ALTER TABLE instances ADD COLUMN guest_observed_at TEXT;
