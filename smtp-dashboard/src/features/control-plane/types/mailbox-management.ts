export type MailboxDetails = {
  id: string;
  tenant_id: string;
  domain_id: string;
  address: string;
  display_name: string;
  kind: "user" | "shared" | string;
  owner_user_id: string | null;
  status: "active" | "suspended" | "deleting" | string;
  used_bytes: number;
  quota_bytes: number;
  created_at: string;
};

export type MailboxFolder = {
  id: string;
  parent_id: string | null;
  name: string;
  special_use: string | null;
  uid_validity: number;
  uid_next: number;
};

export type MailboxAlias = {
  id: string;
  address: string;
  status: string;
  created_at: string;
};

export type MailboxPermission = {
  user_id: string;
  permission: "read" | "read_write" | "manage" | string;
  created_at: string;
};

export type MailboxQuota = {
  mailbox_id: string;
  used_bytes: number;
  quota_bytes: number;
  usage_percent?: number;
};

export type MailboxLabel = {
  id: string;
  name: string;
  kind: "label" | "category" | "tag" | "smart" | string;
  color: string | null;
  query_definition: Record<string, unknown> | null;
};

export type MailboxAppendResult = {
  id: string;
  mailbox_id: string;
  folder_id: string;
  uid: number;
};

export type MailboxMessageResult = {
  id: string;
  folder_id: string;
  uid: number;
};
