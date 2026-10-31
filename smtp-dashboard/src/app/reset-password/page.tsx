import { AuthFrame } from "@/features/auth/components/auth-frame";
import { ResetPasswordForm } from "@/features/auth/components/reset-password-form";

export const instant = false;

export default async function ResetPasswordPage({
  searchParams,
}: PageProps<"/reset-password">) {
  const rawToken = (await searchParams).token;
  const token = Array.isArray(rawToken) ? rawToken[0] ?? "" : rawToken ?? "";
  return (
    <AuthFrame>
      <ResetPasswordForm token={token} />
    </AuthFrame>
  );
}
