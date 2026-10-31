import Link from "next/link";
import type { ReactNode } from "react";

export function AuthFrame({ children }: { children: ReactNode }) {
  return (
    <main className="flex min-h-screen items-center justify-center bg-zinc-50 px-4 py-12">
      <section className="w-full max-w-md rounded-2xl border border-zinc-200 bg-white p-6 shadow-sm sm:p-8">
        {children}
        <div className="mt-6 border-t border-zinc-100 pt-4">
          <Link
            className="text-sm font-medium text-emerald-800 underline underline-offset-4"
            href="/"
          >
            Back to the control plane
          </Link>
        </div>
      </section>
    </main>
  );
}
