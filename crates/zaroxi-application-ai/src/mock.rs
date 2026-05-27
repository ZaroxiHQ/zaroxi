/*
  Removed: application-level mock implementation.

  Rationale:
  - Concrete mock AI adapters belong in the infrastructure layer (`crates/zaroxi-infrastructure-ai-mock`).
  - The application layer should depend only on the `AiClient` port and not ship a concrete mock,
    to preserve separation of concerns and enable swapping infra adapters at composition time.

  If an application-level mock is needed for tests outside of the infra composition,
  wire `zaroxi-infrastructure-ai-mock` into the test composition or provide a lightweight
  test double there. For Phase 10 the infra mock is the canonical implementation.
*/
