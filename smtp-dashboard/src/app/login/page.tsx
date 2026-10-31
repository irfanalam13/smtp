import { AuthFrame } from "@/features/auth/components/auth-frame";
import { OIDCSignIn } from "@/features/auth/components/oidc-sign-in";
import { PasswordLogin } from "@/features/auth/components/password-login";

export default function LoginPage() {
  return (
    <AuthFrame>
      <div className="space-y-6">
        <PasswordLogin />
        <div aria-hidden="true" className="flex items-center gap-3">
          <span className="h-px flex-1 bg-zinc-200" />
          <span className="text-xs font-medium uppercase tracking-wide text-zinc-500">or</span>
          <span className="h-px flex-1 bg-zinc-200" />
        </div>
        <OIDCSignIn />
      </div>
    </AuthFrame>
  );
}
