const MAX_DOMAIN_BYTES = 253;
const MAX_EMAIL_BYTES = 320;
const MAX_SUBJECT_BYTES = 900;
const MAX_RECIPIENTS = 100;
const MAX_TEXT_BYTES = 7 * 1024 * 1024;
const controlCharacters = /[\u0000-\u001f\u007f-\u009f]/;
const emailAddress =
  /^[A-Za-z0-9.!#$%&'*+/=?^_`{|}~-]+@(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\.)+[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?$/;
const domainLabel = /^[\p{L}\p{N}](?:[\p{L}\p{N}-]*[\p{L}\p{N}])?$/u;

function byteLength(value: string): number {
  return new TextEncoder().encode(value).length;
}

export function validateDomainName(value: string): string | null {
  if (!value || value !== value.trim()) {
    return "Enter a domain without leading or trailing spaces.";
  }
  if (byteLength(value) > MAX_DOMAIN_BYTES) {
    return "Domain names must be 253 bytes or fewer.";
  }

  const candidate = value.endsWith(".") ? value.slice(0, -1) : value;
  const labels = candidate.split(".");
  if (
    !candidate.includes(".") ||
    labels.some(
      (label) => !label || byteLength(label) > 63 || !domainLabel.test(label),
    )
  ) {
    return "Enter a valid DNS domain name, such as example.com.";
  }
  return null;
}

function validEmail(value: string): boolean {
  return (
    byteLength(value) <= MAX_EMAIL_BYTES &&
    !/\s/.test(value) &&
    emailAddress.test(value)
  );
}

export type ComposeField = "from" | "to" | "cc" | "bcc" | "subject" | "body";

export type ComposeValidationResult = {
  to: string[];
  cc: string[];
  bcc: string[];
  errors: Partial<Record<ComposeField, string>>;
};

export function validateSubmission(input: {
  from: string;
  to: string;
  cc: string;
  bcc: string;
  subject: string;
  body: string;
}): ComposeValidationResult {
  const errors: ComposeValidationResult["errors"] = {};
  const from = input.from;
  if (!validEmail(from)) {
    errors.from =
      "Enter a valid ASCII email address with no display name or spaces.";
  }

  const parseList = (value: string) =>
    value
      .split(/[,\n;]/)
      .map((address) => address.trim())
      .filter(Boolean);
  const to = parseList(input.to);
  const cc = parseList(input.cc);
  const bcc = parseList(input.bcc);
  const recipients = [...to, ...cc, ...bcc];
  if (recipients.length === 0) {
    errors.to = "Enter between 1 and 100 recipients across To, Cc, and Bcc.";
  } else if (recipients.length > MAX_RECIPIENTS) {
    errors.to = "Use no more than 100 recipients across To, Cc, and Bcc.";
  } else {
    const seen = new Set<string>();
    for (const [field, list] of [
      ["to", to],
      ["cc", cc],
      ["bcc", bcc],
    ] as const) {
      if (list.some((address) => !validEmail(address))) {
        errors[field] = `Each ${field.toUpperCase()} recipient must be a valid ASCII email address.`;
        continue;
      }
      const current = new Set<string>();
      let duplicate = false;
      for (const address of list) {
        const normalized = address.toLowerCase();
        if (seen.has(normalized) || current.has(normalized)) {
          duplicate = true;
        }
        current.add(normalized);
      }
      if (duplicate) {
        errors[field] = "Recipient email addresses must be unique across To, Cc, and Bcc.";
      }
      for (const address of current) {
        seen.add(address);
      }
    }
  }

  if (
    byteLength(input.subject) > MAX_SUBJECT_BYTES ||
    controlCharacters.test(input.subject)
  ) {
    errors.subject =
      "Subject must be 900 bytes or fewer and cannot contain control characters.";
  }
  if (controlCharacters.test(input.body) && input.body.includes("\u0000")) {
    errors.body = "Message text cannot contain a NUL character.";
  } else if (byteLength(input.body) > MAX_TEXT_BYTES) {
    errors.body = "Message text must be 7 MiB or smaller.";
  }

  return { to, cc, bcc, errors };
}
