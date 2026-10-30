package controlplane

import (
	"encoding/json"
	"io"
	"net/http"
	"strings"
)

func JSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

func Error(w http.ResponseWriter, status int, code, message string) {
	JSON(w, status, map[string]any{
		"error": map[string]string{
			"code":       strings.ToUpper(code),
			"message":    message,
			"request_id": w.Header().Get(requestIDHeader),
		},
	})
}

func decodeJSON(r *http.Request, destination any) error {
	if contentType := r.Header.Get("Content-Type"); contentType != "" &&
		!strings.HasPrefix(strings.ToLower(contentType), "application/json") {
		return ErrInvalid
	}
	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(destination); err != nil {
		return err
	}
	var trailing any
	if err := decoder.Decode(&trailing); err != nil {
		if err == io.EOF {
			return nil
		}
		return err
	}
	return ErrInvalid
}
