package controlplane

import "errors"

var (
	ErrUnauthorized            = errors.New("unauthorized")
	ErrTenantSelectionRequired = errors.New("tenant selection required")
	ErrForbidden               = errors.New("forbidden")
	ErrNotFound                = errors.New("not found")
	ErrConflict                = errors.New("conflict")
	ErrInvalid                 = errors.New("invalid request")
	ErrVerificationPending     = errors.New("verification pending")
	ErrRateLimited             = errors.New("rate limited")
	ErrSuppressed              = errors.New("recipient suppressed")
)
