export type DashboardView =
  | "home"
  | "send"
  | "activity"
  | "domains"
  | "mailboxes"
  | "api-keys"
  | "credentials";

export type Domain = {
  id: string;
  domain: string;
  status: string;
};

export type ApiKey = {
  id: string;
  name: string;
  prefix: string;
  scopes: string[];
  revoked_at?: string;
};

export type Mailbox = {
  id: string;
  address: string;
  status: string;
};

export type Message = {
  id: string;
  sender: string;
  subject: string;
  size: number;
  received_at: string;
};

export type DomainCheck = {
  kind: string;
  status: string;
  explanation: string;
  remediation: string;
};

export type Usage = {
  messages_used: number;
  message_limit: number;
  max_message_bytes: number;
};

export type Submission = {
  id: string;
  from: string;
  to: string[];
  subject: string;
  status: string;
  created_at: string;
};

export type SubmissionAttempt = {
  id: number;
  attempt_number: number;
  started_at: string;
  finished_at?: string;
  outcome: string;
  error_category?: string;
  diagnostic?: string;
};

export type Principal = {
  subject: string;
  role: string;
  tenant?: { name?: string };
};

export type VerificationChallenge = {
  record_name: string;
  record_type: string;
  record_value: string;
  secret: string;
};

export type LoadErrors = Partial<
  Record<"domains" | "keys" | "mailboxes" | "usage" | "submissions", string>
>;
