export type DashboardView =
  | "home"
  | "send"
  | "activity"
  | "domains"
  | "mailboxes"
  | "mailbox-admin"
  | "api-keys"
  | "smtp-credentials"
  | "credentials"
  | "statistics"
  | "templates"
  | "sms"
  | "suppressions"
  | "event-webhooks"
  | "routing-rules"
  | "api-reference"
  | "webhooks"
  | "team"
  | "billing";

export type ListResponse<T> = {
  items: T[];
  next_page?: string | null;
};

export type Tenant = {
  id: string;
  name: string;
  status: string;
  created_at: string;
};

export type Principal = {
  subject: string;
  user_id: string;
  tenant: Tenant;
  role: "owner" | "admin" | "member" | string;
};

export type Domain = {
  id: string;
  tenant_id?: string;
  domain: string;
  status: "pending_verification" | "verified" | "disabled" | string;
  created_at: string;
};

export type DomainCheckResult = {
  kind: string;
  status: "pass" | "fail" | "pending" | "warning" | string;
  explanation: string;
  remediation: string;
};

export type DomainCheck = {
  domain_id: string;
  checked_at: string;
  stale: boolean;
  results: DomainCheckResult[];
};

export type DomainReadiness = {
  status: "ready" | "degraded" | "action_required" | "checking";
  checked_at?: string;
  stale?: boolean;
};

export type APIKeyScope =
  | "domains:read"
  | "domains:write"
  | "delivery:read"
  | "delivery:write"
  | "keys:write"
  | "smtp:credentials:read"
  | "smtp:credentials:write"
  | "mailbox:read"
  | "mailbox:write"
  | "mailbox:manage";

export type ApiKey = {
  id: string;
  name: string;
  prefix: string;
  scopes: APIKeyScope[];
  expires_at?: string | null;
  revoked_at?: string | null;
  created_at: string;
};

export type CreatedApiKey = {
  id: string;
  name: string;
  prefix: string;
  scopes: APIKeyScope[];
  secret: string;
  expires_at?: string | null;
  warning: string;
};

export type Mailbox = {
  id: string;
  tenant_id?: string;
  address: string;
  status: string;
  created_at: string;
};

export type Message = {
  id: string;
  mailbox_id: string;
  sender: string;
  subject: string;
  size: number;
  received_at: string;
};

export type Usage = {
  window_started: string;
  window_seconds: number;
  messages_used: number;
  message_limit: number;
  max_message_bytes: number;
};

export type SubmissionStatus =
  | "queued"
  | "processing"
  | "retry_scheduled"
  | "simulated_succeeded"
  | "terminal_failed"
  | "accepted_by_remote"
  | "partially_accepted"
  | "permanently_failed"
  | "delivery_uncertain"
  | "delivery_processing"
  | "cancelled"
  | "suppressed";

export type Submission = {
  id: string;
  from: string;
  to: string[];
  cc: string[];
  bcc: string[];
  subject: string;
  status: SubmissionStatus | string;
  created_at: string;
};

export type SendingConfig = {
  relay_ready: boolean;
  from: string;
  error?: string;
};

export type SubmissionAccepted = {
  submission: Submission;
  created: boolean;
  message: string;
};

export type SubmissionAttempt = {
  id: number;
  attempt_number: number;
  recipient: string;
  started_at: string;
  finished_at?: string | null;
  outcome: "simulated_success" | "failure" | string;
  error_category?: string | null;
  diagnostic?: string | null;
  smtp_code?: number | null;
  remote_host?: string | null;
};

export type VerificationChallenge = {
  record_name: string;
  record_type: string;
  record_value: string;
  secret: string;
};

export type LoadErrors = Partial<
  Record<
    | "principal"
    | "tenant"
    | "readiness"
    | "domains"
    | "keys"
    | "mailboxes"
    | "usage"
    | "submissions",
    string
  >
>;
