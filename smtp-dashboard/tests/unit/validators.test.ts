import { describe, expect, it } from "vitest";
import {
  validateDomainName,
  validateSubmission,
} from "../../src/features/control-plane/utils/validators";

describe("dashboard form validation", () => {
  it("accepts a DNS domain and rejects whitespace or malformed labels", () => {
    expect(validateDomainName("example.com")).toBeNull();
    expect(validateDomainName(" example.com")).toContain("without leading");
    expect(validateDomainName("example..com")).toContain("valid DNS domain");
  });

  it("normalizes recipient separators and rejects duplicate addresses", () => {
    const result = validateSubmission({
      from: "sender@example.com",
      to: " first@example.net ; second@example.net ",
      cc: "",
      bcc: "",
      subject: "Hello",
      body: "Plain text",
    });

    expect(result.to).toEqual([
      "first@example.net",
      "second@example.net",
    ]);
    expect(result.errors).toEqual({});

    const duplicateResult = validateSubmission({
      from: "sender@example.com",
      to: "first@example.net",
      cc: "FIRST@example.net",
      bcc: "",
      subject: "",
      body: "",
    });
    expect(duplicateResult.errors.cc).toContain("must be unique");
  });

  it("validates and keeps To, Cc, and Bcc recipients separately", () => {
    const result = validateSubmission({
      from: "sender@example.com",
      to: "to@example.net",
      cc: "cc@example.net",
      bcc: "bcc@example.net",
      subject: "Hello",
      body: "Plain text",
    });

    expect(result.to).toEqual(["to@example.net"]);
    expect(result.cc).toEqual(["cc@example.net"]);
    expect(result.bcc).toEqual(["bcc@example.net"]);
    expect(result.errors).toEqual({});
  });

  it("rejects unsafe address syntax and invalid control characters", () => {
    const result = validateSubmission({
      from: "Sender <sender@example.com>",
      to: "recipient@example.net",
      cc: "",
      bcc: "",
      subject: "Subject\u0001",
      body: "text\u0000",
    });

    expect(result.errors.from).toContain("ASCII email address");
    expect(result.errors.subject).toContain("control characters");
    expect(result.errors.body).toContain("NUL");
  });
});
