import { notFound, redirect } from "next/navigation";
import ControlPlaneDashboard from "@/features/control-plane/components/shell/control-plane-dashboard";
import { isDashboardPath } from "@/features/control-plane/routing/dashboard-routes";

export const instant = false;

export default async function RoutedDashboard({
  params,
}: PageProps<"/[[...segments]]">) {
  const { segments = [] } = await params;
  if (segments.length === 0) redirect("/home");
  const pathname = `/${segments.join("/")}`;
  if (!isDashboardPath(pathname)) notFound();
  return <ControlPlaneDashboard />;
}
