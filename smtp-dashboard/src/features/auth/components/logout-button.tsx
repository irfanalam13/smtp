"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { requestJson } from "@/lib/api/json-client";

export function LogoutButton() {
  const router = useRouter();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  async function logout() {
    setBusy(true);
    setError("");
    try {
      await requestJson("/v1/auth/logout", { token: "", tenantId: "" }, {
        method: "POST",
      });
      router.replace("/login");
      router.refresh();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Sign-out failed.");
      setBusy(false);
    }
  }

  return (
    <span className="flex items-center gap-2">
      {error && <span className="text-xs text-rose-700" role="alert">{error}</span>}
      <button
        className="button button-quiet"
        disabled={busy}
        onClick={() => void logout()}
        type="button"
      >
        {busy ? "Signing out…" : "Sign out"}
      </button>
    </span>
  );
}
