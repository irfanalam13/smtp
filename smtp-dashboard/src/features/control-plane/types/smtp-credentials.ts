export type SMTPCredential = {
  id: string;
  username: string;
  expires_at: string | null;
  revoked_at: string | null;
  created_at: string;
};

export type CreatedSMTPCredential = {
  id: string;
  username: string;
  authcid: string;
  password: string;
  expires_at: string;
  warning: string;
};
