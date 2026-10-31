import { AuthFrame } from "@/features/auth/components/auth-frame";
import { VerifyEmailForm } from "@/features/auth/components/verify-email-form";

export const instant = false;

export default async function VerifyEmailPage({
  searchParams,
}: PageProps<"/verify-email">) {
  const rawToken = (await searchParams).token;
  const token = Array.isArray(rawToken) ? rawToken[0] ?? "" : rawToken ?? "";
  return (
    <AuthFrame>
      <VerifyEmailForm token={token} />
    </AuthFrame>
  );
}
