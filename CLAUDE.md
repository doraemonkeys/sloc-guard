## CLAUDE.md

1. **Explain "Why", not "What"**: Use comments to explain design rationale, business logic constraints, or non-obvious trade-offs. Code structure and naming should inherently describe the "what."
2. **Design for Testability (DfT)**: Favor Dependency Injection and decoupled components. Define interfaces via Traits to allow easy mocking, and prefer small, pure functions that can be unit-tested in isolation.
3. **Prefer Static Dispatch**: Use Generics and Trait Bounds over Trait Objects (e.g., `Box<dyn Trait>`) to leverage monomorphization and compiler optimizations, unless runtime polymorphism is strictly necessary.
4. **Make Illegal States Unrepresentable**: Use Enums with associated data to model state machines, rather than Structs with many optional fields.
5. **Hard Requirement**: Project CI enforces a **90% minimum test coverage**. Ensure all new features, logic changes, and bug fixes include comprehensive tests to maintain this threshold.
