import { AuthFrame } from "@/features/auth/components/auth-frame";
import { SignupForm } from "@/features/auth/components/signup-form";

export default function SignupPage() {
  return (
    <AuthFrame>
      <SignupForm />
    </AuthFrame>
  );
}
