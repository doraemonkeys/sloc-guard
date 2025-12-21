use super::*;

#[test]
fn test_c_parser() {
    let content = r#"
int main(int argc, char **argv) {
    return 0;
}

static void helper() {
    printf("hello");
}
"#;
    let parser = CParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "main");
    assert_eq!(functions[1].name, "helper");
}

#[test]
fn test_c_parser_pointer_returns() {
    let content = r#"
char* get_string() {
    return "hello";
}

int* allocate_array(size_t n) {
    return malloc(n * sizeof(int));
}

void** get_handles() {
    return handles;
}
"#;
    let parser = CParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "get_string");
    assert_eq!(functions[1].name, "allocate_array");
    assert_eq!(functions[2].name, "get_handles");
}

#[test]
fn test_c_parser_modifiers() {
    let content = r"
inline int fast_add(int a, int b) {
    return a + b;
}

static inline void internal_helper() {
    do_work();
}

extern int library_func(void) {
    return 42;
}
";
    let parser = CParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "fast_add");
    assert_eq!(functions[1].name, "internal_helper");
    assert_eq!(functions[2].name, "library_func");
}

#[test]
fn test_c_parser_complex_signatures() {
    let content = r"
const char* const get_constant() {
    return CONST_STRING;
}

unsigned long long calculate_hash(const char* input) {
    return hash(input);
}

struct Result process_data(struct Input* in) {
    return (struct Result){0};
}
";
    let parser = CParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "get_constant");
    assert_eq!(functions[1].name, "calculate_hash");
    assert_eq!(functions[2].name, "process_data");
}

#[test]
fn test_c_parser_no_false_positives() {
    let content = r"
void real_function() {
    if (condition) {
        do_something();
    }
    while (running) {
        process();
    }
    for (int i = 0; i < 10; i++) {
        iterate();
    }
    switch (value) {
        case 1: break;
    }
}
";
    let parser = CParser::new();
    let functions = parser.parse(content);

    // Should only detect real_function, not if/while/for/switch
    assert_eq!(functions.len(), 1);
    assert_eq!(functions[0].name, "real_function");
}

#[test]
fn test_cpp_parser_class_methods() {
    // C++ out-of-class method definitions with :: are not currently detected
    // by the regex pattern. This documents the current limitation.
    let content = r"
void MyClass::method() {
    this->value = 0;
}

int Container::size() const {
    return m_size;
}
";
    let parser = CParser::new();
    let functions = parser.parse(content);

    // Current parser does not detect namespace-qualified methods
    // This is a known limitation - could be enhanced in the future
    assert_eq!(functions.len(), 0);
}

#[test]
fn test_cpp_parser_inline_class_methods() {
    // Inline methods defined within a class body should be detected
    let content = r"
class MyClass {
    void method() {
        this->value = 0;
    }

    int size() const {
        return m_size;
    }
};
";
    let parser = CParser::new();
    let functions = parser.parse(content);

    // Inline class methods are detected
    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "method");
    assert_eq!(functions[1].name, "size");
}

#[test]
fn test_cpp_parser_modern_features() {
    let content = r"
auto deduce_type() {
    return 42;
}

constexpr int compile_time() {
    return 100;
}

void noexcept_func() noexcept {
    safe_operation();
}

void override_func() override {
    derived_impl();
}
";
    let parser = CParser::new();
    let functions = parser.parse(content);

    assert!(functions.len() >= 2);
}

#[test]
fn test_cpp_parser_templates() {
    let content = r"
template<typename T>
T max_value(T a, T b) {
    return a > b ? a : b;
}

void regular_after_template() {
    do_work();
}
";
    let parser = CParser::new();
    let functions = parser.parse(content);

    // At minimum, should detect regular_after_template
    assert!(!functions.is_empty());
    assert!(functions.iter().any(|f| f.name == "regular_after_template"));
}
