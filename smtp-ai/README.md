# smtp-ai

Reserved for opt-in AI-assisted platform features. No AI runtime or provider
integration is included. The shared-types crate now provides a deterministic,
offline email feature extractor over the bounded MIME projection; it emits
aggregate counts only and does not classify messages or make model calls. The
shared-types crate also provides a bounded, review-only draft suggestion
contract with source-message and model-version provenance; it does not send
mail or make model calls. It also exposes a bounded in-memory cosine-ranking
primitive for caller-provided embeddings and an explicit tenant/mailbox scope;
it does not generate embeddings, retrieve mail, persist vectors, or replace
authorization checks.
