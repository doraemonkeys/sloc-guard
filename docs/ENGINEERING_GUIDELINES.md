# ENGINEERING GUIDELINES

### Code Readability
* Use meaningful variable and function names.
* Comments must be added only when necessary.
* Use comments to explain "why," not "what." Good code is self-documenting and explains what it does. Comments should be reserved for explaining design decisions or complex logic.
* Avoid clutter. Do not write obvious comments, such as `i++ // Increment i by 1`.
* Avoid Hardcoding: Extract unexplained numeric and string values into named constants.


### Best Practices
* Break down complex problems into smaller, manageable parts
* Consider performance implications


### Design for Testability
* No Direct Instantiation: Prohibit instantiating external dependencies directly inside functions (DB, API clients, etc.) .
* Dependency Injection: Ensure all dependencies are provided externally via the constructor or method parameters.
* Dependency Inversion: Define Interfaces for all external dependencies; business logic must rely on these abstractions rather than concrete implementations.
* Avoid Global State: Ban the use of Singletons or global variables unless absolutely necessary and properly encapsulated, as they impede test isolation.


### Rust Specific
Follow standard Rust idioms (The "Rust way").

* Implement standard traits (like `Debug`, `Default`, `Display`, `From`) for your types where appropriate.
* Don't name modules util, common, or misc. Organize modules by domain/feature (what they provide), not by file type.
* Make Illegal States Unrepresentable: Use Enums with associated data to model state machines, rather than Structs with many optional fields.
* Prefer Generics and Trait Bounds (Static Dispatch) over Trait Objects (`Box<dyn Trait>`) unless dynamic dispatch is required.
* Accept Traits, Return Concrete Types: Accept arguments as Generics or impl Trait (consumer defines requirements), but return concrete structs/enums (producer defines implementation).
* Avoid unnecessary .clone() unless required by ownership logic.
* Minimize the use of `Rc`/`Arc`/`RefCell`/`Mutex` unless shared ownership or interior mutability is strictly necessary.
* Use `#[allow()]` sparingly. Avoid bypassing CI lint checks unless there's a justified reason documented in a comment.


### Design Principles
* Principle of Least Surprise: Design logic to be intuitive. Code implementation must behave as a developer expects, and functional design must align with the user's intuition.
* Logical Completeness: Prioritize first-principles domain modeling and logical orthogonality; favor refactoring core structures to capture native semantics over adding additive flags or 'patch' parameters.
* No Backward Compatibility: Prioritize architectural correctness over legacy support. You are free to break old formats if it results in a cleaner design.
* Refactoring Circuit Breaker: If achieving the ideal structure requires a massive, high-risk rewrite (e.g., changing core assumptions), STOP and explain the scope and complexity first. 