import { AuthFrame } from "@/features/auth/components/auth-frame";
import { ForgotPasswordForm } from "@/features/auth/components/forgot-password-form";

export default function ForgotPasswordPage() {
  return (
    <AuthFrame>
      <ForgotPasswordForm />
    </AuthFrame>
  );
}
