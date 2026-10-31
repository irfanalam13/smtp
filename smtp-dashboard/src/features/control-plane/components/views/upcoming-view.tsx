import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { CodeBlock } from "@/components/ui/code-block";
import { FileTextIcon, KeyIcon, ServerIcon } from "@/components/ui/icons";
import type { DashboardView } from "../../types/control-plane";

interface UpcomingViewProps {
  view: DashboardView;
  onNavigate: (view: DashboardView) => void;
}

interface FeatureMetadata {
  title: string;
  tagline: string;
  description: string;
  category: string;
  status: "Planned" | "In Development" | "Beta";
  estimatedMilestone: string;
  keyCapabilities: string[];
  plannedEndpoints: {
    method: "GET" | "POST" | "DELETE" | "PUT";
    path: string;
    summary: string;
  }[];
  sdkSnippet: string;
  suggestedAction?: { label: string; view: DashboardView };
}

const upcomingFeatures: Record<string, FeatureMetadata> = {
  statistics: {
    title: "Analytics & Statistics",
    tagline:
      "Real-time metrics, deliverability rates, and bounce intelligence.",
    description:
      "Deep aggregate metrics tracking delivery rate, bounce classifications (soft vs hard), spam complaints, latency distributions, and ISP-specific reputations.",
    category: "Transaction",
    status: "In Development",
    estimatedMilestone: "Phase 4 - Observability & Metrics",
    keyCapabilities: [
      "Time-series delivery volume (Hourly, Daily, 30-day)",
      "ISP deliverability breakdown (Gmail, Outlook, Yahoo, Corporate)",
      "Bounce categorization & feedback loop tracking",
      "SMTP connection latency and queue wait-time metrics",
    ],
    plannedEndpoints: [
      {
        method: "GET",
        path: "/v1/analytics/overview",
        summary: "Aggregate delivery metrics",
      },
      {
        method: "GET",
        path: "/v1/analytics/time-series",
        summary: "Volume and latency buckets",
      },
      {
        method: "GET",
        path: "/v1/analytics/reputation",
        summary: "Domain sender reputation scores",
      },
    ],
    sdkSnippet: `// Query delivery statistics with the Go / TypeScript SDK
const stats = await client.analytics.getOverview({
  range: "last_7_days",
  metrics: ["delivered", "bounced", "opened", "complained"],
});
console.log(\`Delivery rate: \${stats.deliveryRate}%\`);`,
    suggestedAction: { label: "Inspect Live Activity", view: "activity" },
  },
  templates: {
    title: "Email Templates",
    tagline: "Dynamic responsive templates with variable substitution.",
    description:
      "Create, preview, and version transactional email templates using Handlebars or Liquid syntax with server-side variable compilation.",
    category: "Transaction",
    status: "Planned",
    estimatedMilestone: "Phase 4 - Platform Capabilities",
    keyCapabilities: [
      "Visual split-pane preview (Desktop & Mobile)",
      "Handlebars & Liquid parameter syntax checking",
      "Version history with instant rollback",
      "Draft vs Published deployment lifecycle",
    ],
    plannedEndpoints: [
      {
        method: "GET",
        path: "/v1/templates",
        summary: "List active templates",
      },
      {
        method: "POST",
        path: "/v1/templates",
        summary: "Create or version a template",
      },
      {
        method: "POST",
        path: "/v1/templates/{id}/preview",
        summary: "Render with sample variables",
      },
    ],
    sdkSnippet: `// Send an email with a compiled template
await client.submissions.send({
  templateId: "tmpl_welcome_onboarding",
  to: ["user@company.com"],
  variables: {
    name: "Alex",
    confirmationUrl: "https://app.domain.com/confirm?token=xyz",
  },
});`,
    suggestedAction: { label: "Send Test Email", view: "send" },
  },
  sms: {
    title: "SMS Messaging",
    tagline: "High-throughput transactional SMS delivery.",
    description:
      "Multi-channel transactional notifications with carrier routing, automatic shortcode/toll-free balancing, and delivery receipts.",
    category: "Transaction",
    status: "Planned",
    estimatedMilestone: "Phase 5 - Multi-channel Extensions",
    keyCapabilities: [
      "10DLC and Toll-free carrier compliant routing",
      "Real-time handset delivery receipts (DLR)",
      "Automatic opt-out (STOP/UNSUBSCRIBE) enforcement",
      "Global country prefix formatting and validation",
    ],
    plannedEndpoints: [
      {
        method: "POST",
        path: "/v1/sms/messages",
        summary: "Queue outbound SMS",
      },
      {
        method: "GET",
        path: "/v1/sms/numbers",
        summary: "List configured sender numbers",
      },
      {
        method: "GET",
        path: "/v1/sms/receipts/{id}",
        summary: "Get carrier delivery receipt",
      },
    ],
    sdkSnippet: `// Send an urgent 2FA SMS
await client.sms.send({
  to: "+15550192834",
  body: "Your verification code is 849201. It expires in 5 minutes.",
});`,
    suggestedAction: { label: "Go to API Keys", view: "api-keys" },
  },
  suppressions: {
    title: "Suppressions & Bounces",
    tagline: "Automated list hygiene protecting sender reputation.",
    description:
      "Automatic and manual suppression lists preventing sends to hard-bounced addresses, spam complainers, and unsubscribed contacts.",
    category: "Deliverability",
    status: "In Development",
    estimatedMilestone: "Phase 3 - Deliverability & Hygiene",
    keyCapabilities: [
      "Automated hard bounce detection and blocking",
      "Spam complaint feedback loop (FBL) ingestion",
      "Custom regex or wildcard exclusion rules",
      "Audit trail for suppression additions and removals",
    ],
    plannedEndpoints: [
      {
        method: "GET",
        path: "/v1/suppressions",
        summary: "List active suppressions",
      },
      {
        method: "POST",
        path: "/v1/suppressions",
        summary: "Add address to suppression list",
      },
      {
        method: "DELETE",
        path: "/v1/suppressions/{email}",
        summary: "Remove address after verification",
      },
    ],
    sdkSnippet: `// Check or add suppression
const isSuppressed = await client.suppressions.check("bounced@example.com");
if (isSuppressed) {
  console.log("Recipient is currently suppressed to protect reputation.");
}`,
    suggestedAction: { label: "Sending Domains", view: "domains" },
  },
  "event-webhooks": {
    title: "Event Webhooks",
    tagline: "Real-time delivery lifecycle webhooks with HMAC signatures.",
    description:
      "Stream raw delivery events (delivered, bounced, deferred, opened, clicked) to your HTTP endpoints with exponential retry and signing secrets.",
    category: "Deliverability",
    status: "Planned",
    estimatedMilestone: "Phase 4 - Real-time Integration",
    keyCapabilities: [
      "Cryptographic HMAC-SHA256 signature verification",
      "Configurable event subscription topics",
      "Automatic retry with exponential backoff on 5xx failures",
      "Dead-letter replay queue for failed webhook attempts",
    ],
    plannedEndpoints: [
      {
        method: "GET",
        path: "/v1/webhooks/deliverability",
        summary: "List webhook subscriptions",
      },
      {
        method: "POST",
        path: "/v1/webhooks/deliverability",
        summary: "Register webhook endpoint",
      },
      {
        method: "POST",
        path: "/v1/webhooks/test",
        summary: "Send ping test event",
      },
    ],
    sdkSnippet: `// Webhook payload example received by your server
{
  "event": "message.delivered",
  "submissionId": "sub_01HNG72K3M90A",
  "recipient": "user@customer.com",
  "timestamp": "2026-10-30T16:30:00Z",
  "smtpResponse": "250 2.0.0 OK: queued as 482103"
}`,
    suggestedAction: { label: "API Keys", view: "api-keys" },
  },
  "routing-rules": {
    title: "Inbound Routing Rules",
    tagline: "Rule-based filtering, catch-all domains, and MX webhooks.",
    description:
      "Inspect inbound messages delivered to your verified MX records, execute regex matching rules, and route them to mailboxes or HTTP endpoints.",
    category: "Inbound",
    status: "In Development",
    estimatedMilestone: "Phase 3 - Mailbox & Inbound",
    keyCapabilities: [
      "Catch-all address support for custom domains",
      "Regex local-part routing (e.g. support+*@domain.com)",
      "Forwarding to HTTP webhooks or IMAP/POP3 storage",
      "Spam filtering score thresholds",
    ],
    plannedEndpoints: [
      {
        method: "GET",
        path: "/v1/inbound/rules",
        summary: "List routing rules",
      },
      {
        method: "POST",
        path: "/v1/inbound/rules",
        summary: "Create match pattern and target",
      },
    ],
    sdkSnippet: `// Configure inbound catch-all forwarding
await client.inbound.createRule({
  domain: "mycompany.com",
  pattern: "support+*@mycompany.com",
  action: { forwardToMailbox: "mb_support_team" }
});`,
    suggestedAction: { label: "Mailbox Management", view: "mailbox-admin" },
  },
  "api-reference": {
    title: "API Reference & SDKs",
    tagline: "Complete REST API specifications and client libraries.",
    description:
      "Full OpenAPI 3.1 specification, Go SDK (`smtp-sdk`), Rust crates, TypeScript bindings, and CLI documentation.",
    category: "Developers",
    status: "Beta",
    estimatedMilestone: "Active - Core Backend Implemented",
    keyCapabilities: [
      "Interactive Swagger/OpenAPI endpoint documentation",
      "Idempotency-Key duplicate prevention support",
      "Bearer Token and X-Tenant-ID header specifications",
      "Official SDKs for Go, Rust, Python, and TypeScript",
    ],
    plannedEndpoints: [
      {
        method: "GET",
        path: "/docs/api/openapi.yaml",
        summary: "OpenAPI 3.1 Contract",
      },
      { method: "GET", path: "/v1/me", summary: "Current Principal" },
      { method: "GET", path: "/v1/tenants/current", summary: "Current Tenant" },
      { method: "POST", path: "/v1/submissions", summary: "Submit Email" },
    ],
    sdkSnippet: `// cURL example against the control plane
curl -X POST http://127.0.0.1:8080/v1/submissions \\
  -H "Authorization: Bearer <API_KEY>" \\
  -H "Content-Type: application/json" \\
  -d '{"from":"you@verified.com","to":["recipient@test.com"],"subject":"Hello","body":"Testing"}'`,
    suggestedAction: { label: "Manage API Keys", view: "api-keys" },
  },
  webhooks: {
    title: "Developer Webhooks",
    tagline: "Manage developer endpoints for queue and account events.",
    description:
      "Configure developer notification endpoints for API key lifecycle changes, quota warnings, and system alerts.",
    category: "Developers",
    status: "Planned",
    estimatedMilestone: "Phase 4 - Developer Platform",
    keyCapabilities: [
      "Secret key rotation without downtime",
      "Event filtering by resource ID",
      "Delivery logs with full request/response bodies",
    ],
    plannedEndpoints: [
      {
        method: "GET",
        path: "/v1/developer/webhooks",
        summary: "List webhooks",
      },
      {
        method: "POST",
        path: "/v1/developer/webhooks",
        summary: "Create webhook",
      },
    ],
    sdkSnippet: `// Developer webhook signing verification
const isValid = verifySignature({
  payload: requestBody,
  signature: headers["x-webhook-signature"],
  secret: process.env.WEBHOOK_SECRET,
});`,
    suggestedAction: { label: "API Keys", view: "api-keys" },
  },
  team: {
    title: "Team & Access Control",
    tagline: "Role-based access control and collaborative workspaces.",
    description:
      "Invite team members, assign fine-grained roles (Owner, Admin, Member, Operator), and configure single sign-on (OIDC/SAML).",
    category: "Account",
    status: "Planned",
    estimatedMilestone: "Phase 5 - Enterprise Identity",
    keyCapabilities: [
      "Role-based permission enforcement (`role:owner`, `role:admin`)",
      "Multi-tenant isolation and tenant switching",
      "Audit logging of team administrative actions",
    ],
    plannedEndpoints: [
      {
        method: "GET",
        path: "/v1/team/members",
        summary: "List workspace members",
      },
      {
        method: "POST",
        path: "/v1/team/invites",
        summary: "Invite new user by email",
      },
    ],
    sdkSnippet: `// Invite a developer to your tenant
await client.team.inviteMember({
  email: "developer@company.com",
  role: "admin",
  scopes: ["delivery:write", "domains:read"]
});`,
    suggestedAction: { label: "Credentials View", view: "credentials" },
  },
  billing: {
    title: "Usage & Billing",
    tagline: "Message consumption meters, quota tiers, and invoicing.",
    description:
      "Monitor send rates, provisioned mailboxes, storage consumption, and upgrade your sending limits.",
    category: "Account",
    status: "In Development",
    estimatedMilestone: "Phase 3 - Quota & Metering",
    keyCapabilities: [
      "Daily and monthly submission count tracking",
      "Mailbox quota and storage usage meters",
      "Over-quota alert notifications",
    ],
    plannedEndpoints: [
      {
        method: "GET",
        path: "/v1/usage",
        summary: "Get current tenant consumption",
      },
      {
        method: "GET",
        path: "/v1/billing/subscription",
        summary: "Active tier details",
      },
    ],
    sdkSnippet: `// Inspect current usage
const usage = await client.usage.get();
console.log(\`Sent today: \${usage.submissions_today} / \${usage.daily_quota}\`);`,
    suggestedAction: { label: "Back to Home", view: "home" },
  },
};

export function UpcomingView({ view, onNavigate }: UpcomingViewProps) {
  const meta = upcomingFeatures[view] || {
    title: "Upcoming Feature",
    tagline: "This capability is currently scheduled on the platform roadmap.",
    description:
      "The API and dashboard UI for this feature are being actively developed.",
    category: "Platform",
    status: "Planned",
    estimatedMilestone: "Upcoming Phase",
    keyCapabilities: [
      "Full API integration",
      "Responsive dashboard UI",
      "Typed SDK support",
    ],
    plannedEndpoints: [],
    sdkSnippet: `// Feature roadmap preview\nconsole.log("Coming soon");`,
  };

  return (
    <div className="space-y-6">
      {/* Banner Card */}
      <Card className="border-emerald-200/60 bg-gradient-to-br from-white via-emerald-50/20 to-white">
        <CardHeader>
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex items-center gap-2">
              <Badge variant="neutral">{meta.category}</Badge>
              <Badge
                variant={meta.status === "In Development" ? "warning" : "info"}
                withDot
              >
                {meta.status}
              </Badge>
            </div>
            <span className="text-xs font-medium text-emerald-800 bg-emerald-100/60 px-3 py-1 rounded-full">
              {meta.estimatedMilestone}
            </span>
          </div>
          <CardTitle className="text-xl sm:text-2xl mt-2 text-zinc-900">
            {meta.title}
          </CardTitle>
          <CardDescription className="text-sm font-medium text-zinc-700">
            {meta.tagline}
          </CardDescription>
          <p className="text-xs text-zinc-500 mt-1 max-w-3xl leading-relaxed">
            {meta.description}
          </p>
        </CardHeader>
        {meta.suggestedAction && (
          <CardContent className="pt-0">
            <div className="flex items-center gap-3">
              <Button
                variant="primary"
                size="sm"
                onClick={() => onNavigate(meta.suggestedAction!.view)}
              >
                {meta.suggestedAction.label} &rarr;
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => onNavigate("home")}
              >
                Return to Dashboard
              </Button>
            </div>
          </CardContent>
        )}
      </Card>

      {/* Grid of Capabilities & Architecture */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Planned Capabilities */}
        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <ServerIcon className="h-4 w-4 text-emerald-600" />
              <span>Core Capabilities</span>
            </CardTitle>
            <CardDescription>
              Architected specifications defined for this feature.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <ul className="space-y-2.5">
              {meta.keyCapabilities.map((cap, idx) => (
                <li
                  key={idx}
                  className="flex items-start gap-2.5 text-xs text-zinc-700"
                >
                  <span className="h-4 w-4 rounded-full bg-emerald-100 text-emerald-700 flex items-center justify-center shrink-0 text-[10px] font-bold mt-0.5">
                    ✓
                  </span>
                  <span>{cap}</span>
                </li>
              ))}
            </ul>
          </CardContent>
        </Card>

        {/* REST API Endpoints Planned */}
        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <KeyIcon className="h-4 w-4 text-emerald-600" />
              <span>API Contract Endpoints</span>
            </CardTitle>
            <CardDescription>
              Backend Go handlers and OpenAPI operations mapped to this area.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {meta.plannedEndpoints.map((ep, idx) => (
                <div
                  key={idx}
                  className="flex items-center justify-between p-2 rounded-lg bg-zinc-50 border border-zinc-100 text-xs"
                >
                  <div className="flex items-center gap-2">
                    <span
                      className={`px-1.5 py-0.5 rounded text-[10px] font-mono font-bold ${
                        ep.method === "POST"
                          ? "bg-blue-100 text-blue-700"
                          : ep.method === "GET"
                            ? "bg-emerald-100 text-emerald-700"
                            : "bg-rose-100 text-rose-700"
                      }`}
                    >
                      {ep.method}
                    </span>
                    <code className="text-zinc-800 font-mono text-[11px]">
                      {ep.path}
                    </code>
                  </div>
                  <span className="text-[11px] text-zinc-500 hidden sm:inline">
                    {ep.summary}
                  </span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Code Snippet Preview */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <FileTextIcon className="h-4 w-4 text-emerald-600" />
            <span>Developer Code Sample</span>
          </CardTitle>
          <CardDescription>
            Example integration using the typed SMTP Platform client.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <CodeBlock code={meta.sdkSnippet} label="typescript / node.js" />
        </CardContent>
      </Card>
    </div>
  );
}
