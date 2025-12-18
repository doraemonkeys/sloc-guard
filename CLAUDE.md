## CLAUDE.md

### Code Readability
* Use meaningful variable and function names.
* Comments must be added only when necessary.
* Use comments to explain "why," not "what." Good code is self-documenting and explains what it does. Comments should be reserved for explaining design decisions or complex logic.
* Avoid clutter. Do not write obvious comments, such as `i++ // Increment i by 1`.

### Development Process
* First think through the problem before writing code
* Consider the first principles and fundamental requirements
* Plan the structure and architecture beforehand
* Break down complex problems into smaller, manageable parts

### Best Practices
* Follow language-specific conventions and style guides
* Don't ignore errors
* Write maintainable and scalable code
* Consider performance implications

### Design for Testability
* No Direct Instantiation: Prohibit instantiating external dependencies directly inside functions (DB, API clients, etc.) .
* Dependency Injection: Ensure all dependencies are provided externally via the constructor or method parameters.
* Dependency Inversion: Define Interfaces for all external dependencies; business logic must rely on these abstractions rather than concrete implementations.
* Avoid Global State: Ban the use of Singletons or global variables unless absolutely necessary and properly encapsulated, as they impede test isolation.

### Rust Specific
Follow standard Rust idioms (The "Rust way").

* Implement standard traits (like `Debug`, `Default`, `Display`, `From`/`Into`) for your types where appropriate.
* Don't name modules util, common, or misc. Organize modules by domain/feature (what they provide), not by file type.
* Make Illegal States Unrepresentable: Use Enums with associated data to model state machines, rather than Structs with many optional fields.
* Prefer Generics and Trait Bounds (Static Dispatch) over Trait Objects (`Box<dyn Trait>`) unless dynamic dispatch is required.
* Accept Traits, Return Concrete Types: Accept arguments as Generics or impl Trait (consumer defines requirements), but return concrete structs/enums (producer defines implementation).
* Avoid unnecessary .clone() unless required by ownership logic.

### Other Rules
* No backward compatibility: Break old formats freely