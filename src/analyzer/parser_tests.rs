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
fn test_go_parser() {
    let content = r#"
func SimpleFunc() {
    fmt.Println("hello")
}

func (s *Server) Method() {
    s.handle()
}
"#;
    let parser = GoParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "SimpleFunc");
    assert_eq!(functions[1].name, "Method");
}

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
fn test_js_parser() {
    let content = r#"
function simpleFunc() {
    console.log("hello");
}

export async function asyncFunc() {
    await fetch();
}

const arrowFunc = async () => {
    return 1;
};

class MyClass {
    constructor() {}
}
"#;
    let parser = JsParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 4);
    assert_eq!(functions[0].name, "simpleFunc");
    assert_eq!(functions[1].name, "asyncFunc");
    assert_eq!(functions[2].name, "arrowFunc");
    assert_eq!(functions[3].name, "MyClass");
}

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

