use super::*;

#[test]
fn test_rust_parser() {
    let content = r#"
fn simple_fn() {
    println!("hello");
}

pub fn public_fn() {
    let x = 1;
}

pub async fn async_fn() {
    tokio::spawn(async {});
}

impl Foo {
    pub fn method(&self) {
        self.x;
    }
}
"#;
    let parser = RustParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 4);
    assert_eq!(functions[0].name, "simple_fn");
    assert_eq!(functions[1].name, "public_fn");
    assert_eq!(functions[2].name, "async_fn");
    assert_eq!(functions[3].name, "method");
}

#[test]
fn test_rust_parser_visibility_modifiers() {
    let content = r"
pub(crate) fn crate_visible() {
    do_something();
}

pub(super) fn super_visible() {
    do_something();
}

pub(in crate::module) fn path_visible() {
    do_something();
}
";
    let parser = RustParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "crate_visible");
    assert_eq!(functions[1].name, "super_visible");
    assert_eq!(functions[2].name, "path_visible");
}

#[test]
fn test_rust_parser_unsafe_and_const() {
    let content = r"
unsafe fn dangerous() {
    std::ptr::null();
}

const fn compile_time() -> u32 {
    42
}

pub unsafe fn public_unsafe() {
    std::mem::transmute(0u8);
}

pub const fn public_const() -> bool {
    true
}
";
    let parser = RustParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 4);
    assert_eq!(functions[0].name, "dangerous");
    assert_eq!(functions[1].name, "compile_time");
    assert_eq!(functions[2].name, "public_unsafe");
    assert_eq!(functions[3].name, "public_const");
}

#[test]
fn test_rust_parser_generics_and_lifetimes() {
    let content = r"
fn generic<T>(value: T) -> T {
    value
}

fn with_lifetime<'a>(s: &'a str) -> &'a str {
    s
}

fn complex<'a, T: Clone + Debug>(items: &'a [T]) -> Vec<T> {
    items.to_vec()
}

pub fn where_clause<T>(x: T) -> T
where
    T: Clone + Default,
{
    x.clone()
}
";
    let parser = RustParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 4);
    assert_eq!(functions[0].name, "generic");
    assert_eq!(functions[1].name, "with_lifetime");
    assert_eq!(functions[2].name, "complex");
    assert_eq!(functions[3].name, "where_clause");
}

#[test]
fn test_rust_parser_trait_impl() {
    let content = r#"
impl Display for MyType {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Clone> Iterator for MyIter<T> {
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
"#;
    let parser = RustParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "fmt");
    assert_eq!(functions[1].name, "next");
}

#[test]
fn test_rust_parser_async_unsafe_combination() {
    let content = r"
pub async unsafe fn async_unsafe_fn() {
    dangerous_async_op().await;
}

async fn private_async() {
    sleep(Duration::from_secs(1)).await;
}
";
    let parser = RustParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "async_unsafe_fn");
    assert_eq!(functions[1].name, "private_async");
}

#[test]
fn test_rust_parser_nested_braces() {
    let content = r"
fn with_nested_blocks() {
    {
        let inner = {
            compute()
        };
    }
    match value {
        Some(x) => { process(x) }
        None => {}
    }
}

fn after_nested() {
    simple();
}
";
    let parser = RustParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "with_nested_blocks");
    assert_eq!(functions[1].name, "after_nested");
    // Verify block end detection works correctly
    assert!(functions[0].end_line < functions[1].start_line);
}

#[test]
fn test_line_number_accuracy() {
    let content = r"fn first() {
    line_2();
}

fn second() {
    line_6();
    line_7();
}
";
    let parser = RustParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "first");
    assert_eq!(functions[0].start_line, 1);
    assert_eq!(functions[0].end_line, 3);

    assert_eq!(functions[1].name, "second");
    assert_eq!(functions[1].start_line, 5);
    assert_eq!(functions[1].end_line, 8);
}
