package smtpauth

import (
	"crypto/rand"
	"crypto/subtle"
	"encoding/base64"
	"errors"
	"fmt"
	"strconv"
	"strings"

	"golang.org/x/crypto/argon2"
)

var (
	ErrPasswordCapacity = errors.New("password hashing capacity is temporarily exhausted")
	passwordSlots       = make(chan struct{}, 2)
)

func HashPasswordBounded(password string) (string, error) {
	if !acquirePasswordSlot() {
		return "", ErrPasswordCapacity
	}
	defer releasePasswordSlot()
	return HashPassword(password)
}

func VerifyPasswordBounded(encoded, password string) (bool, error) {
	if !acquirePasswordSlot() {
		return false, ErrPasswordCapacity
	}
	defer releasePasswordSlot()
	return VerifyPassword(encoded, password)
}

func acquirePasswordSlot() bool {
	select {
	case passwordSlots <- struct{}{}:
		return true
	default:
		return false
	}
}

func releasePasswordSlot() {
	<-passwordSlots
}

const (
	argonMemory      = 64 * 1024
	argonIterations  = 3
	argonParallelism = 2
	argonSaltBytes   = 16
	argonKeyBytes    = 32
)

func HashPassword(password string) (string, error) {
	if len(password) < 12 || len(password) > 1024 {
		return "", errors.New("password length must be between 12 and 1024 bytes")
	}
	salt := make([]byte, argonSaltBytes)
	if _, err := rand.Read(salt); err != nil {
		return "", fmt.Errorf("generate password salt: %w", err)
	}
	key := argon2.IDKey([]byte(password), salt, argonIterations, argonMemory, argonParallelism, argonKeyBytes)
	return fmt.Sprintf(
		"$argon2id$v=19$m=%d,t=%d,p=%d$%s$%s",
		argonMemory, argonIterations, argonParallelism,
		base64.RawStdEncoding.EncodeToString(salt),
		base64.RawStdEncoding.EncodeToString(key),
	), nil
}

func VerifyPassword(encoded, password string) (bool, error) {
	if len(encoded) > 512 || len(password) > 1024 {
		return false, errors.New("password verifier or password exceeds supported size")
	}
	parts := strings.Split(encoded, "$")
	if len(parts) != 6 || parts[0] != "" || parts[1] != "argon2id" || parts[2] != "v=19" {
		return false, errors.New("invalid Argon2id password verifier")
	}
	parameters := strings.Split(parts[3], ",")
	if len(parameters) != 3 {
		return false, errors.New("invalid Argon2id parameters")
	}
	memory, err := parseParameter(parameters[0], "m=")
	if err != nil || memory < 8*1024 || memory > 256*1024 {
		return false, errors.New("invalid Argon2id memory parameter")
	}
	iterations, err := parseParameter(parameters[1], "t=")
	if err != nil || iterations < 1 || iterations > 10 {
		return false, errors.New("invalid Argon2id iteration parameter")
	}
	parallelism, err := parseParameter(parameters[2], "p=")
	if err != nil || parallelism < 1 || parallelism > 8 {
		return false, errors.New("invalid Argon2id parallelism parameter")
	}
	salt, err := base64.RawStdEncoding.DecodeString(parts[4])
	if err != nil || len(salt) < 16 || len(salt) > 64 {
		return false, errors.New("invalid Argon2id salt")
	}
	expected, err := base64.RawStdEncoding.DecodeString(parts[5])
	if err != nil || len(expected) < 16 || len(expected) > 64 {
		return false, errors.New("invalid Argon2id hash")
	}
	actual := argon2.IDKey([]byte(password), salt, uint32(iterations), uint32(memory), uint8(parallelism), uint32(len(expected)))
	return subtle.ConstantTimeCompare(actual, expected) == 1, nil
}

func parseParameter(parameter, prefix string) (int, error) {
	if !strings.HasPrefix(parameter, prefix) {
		return 0, errors.New("invalid Argon2id parameter")
	}
	return strconv.Atoi(strings.TrimPrefix(parameter, prefix))
}
