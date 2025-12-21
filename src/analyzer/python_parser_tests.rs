use super::*;

#[test]
fn test_python_parser() {
    let content = r#"
def simple_fn():
    print("hello")

class MyClass:
    def method(self):
        pass

async def async_fn():
    await something()
"#;
    let parser = PythonParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "simple_fn");
    assert_eq!(functions[1].name, "MyClass");
    assert_eq!(functions[2].name, "async_fn");
}

#[test]
fn test_python_parser_decorated_functions() {
    let content = r"
@decorator
def decorated():
    pass

@decorator_with_args(arg1, arg2)
def decorated_with_args():
    pass

@functools.lru_cache(maxsize=128)
def cached_function(x):
    return expensive_computation(x)
";
    let parser = PythonParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "decorated");
    assert_eq!(functions[1].name, "decorated_with_args");
    assert_eq!(functions[2].name, "cached_function");
}

#[test]
fn test_python_parser_class_methods() {
    let content = r"
class MyClass:
    def __init__(self):
        self.value = 0

    @classmethod
    def from_string(cls, s):
        return cls()

    @staticmethod
    def utility():
        return True

    @property
    def computed(self):
        return self.value * 2
";
    let parser = PythonParser::new();
    let functions = parser.parse(content);

    // Only top-level class should be detected
    assert_eq!(functions.len(), 1);
    assert_eq!(functions[0].name, "MyClass");
}

#[test]
fn test_python_parser_with_docstrings() {
    let content = r#"
def with_docstring():
    """This is a docstring."""
    pass

def multiline_docstring():
    """
    This is a multiline docstring.
    It spans multiple lines.
    """
    return 42

class DocumentedClass:
    """Class docstring."""
    pass
"#;
    let parser = PythonParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "with_docstring");
    assert_eq!(functions[1].name, "multiline_docstring");
    assert_eq!(functions[2].name, "DocumentedClass");
}

#[test]
fn test_python_parser_type_hints() {
    let content = r#"
def typed_function(x: int, y: str) -> bool:
    return len(y) > x

def complex_types(items: List[Dict[str, Any]]) -> Optional[Result]:
    return None

async def async_typed(data: bytes) -> AsyncIterator[str]:
    yield "chunk"
"#;
    let parser = PythonParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "typed_function");
    assert_eq!(functions[1].name, "complex_types");
    assert_eq!(functions[2].name, "async_typed");
}

#[test]
fn test_python_parser_multiline_signature() {
    let content = r"
def long_signature(
    arg1: int,
    arg2: str,
    arg3: Optional[List[int]] = None,
) -> Dict[str, Any]:
    return {}

class ConfiguredClass(
    BaseClass,
    Mixin1,
    Mixin2,
):
    pass
";
    let parser = PythonParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "long_signature");
    assert_eq!(functions[1].name, "ConfiguredClass");
}

#[test]
fn test_python_parser_dunder_methods() {
    let content = r"
def __main__():
    run()

class Container:
    def __len__(self):
        return 0

    def __getitem__(self, key):
        return None

    def __iter__(self):
        return iter([])
";
    let parser = PythonParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "__main__");
    assert_eq!(functions[1].name, "Container");
}

#[test]
fn test_python_parser_nested_functions() {
    let content = r"
def outer():
    def inner():
        return 1
    return inner()

def another_top_level():
    pass
";
    let parser = PythonParser::new();
    let functions = parser.parse(content);

    // Should only detect top-level functions
    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "outer");
    assert_eq!(functions[1].name, "another_top_level");
}
