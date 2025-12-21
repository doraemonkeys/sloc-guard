use super::*;

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
fn test_go_parser_special_functions() {
    let content = r"
func init() {
    setup()
}

func main() {
    run()
}

func TestSomething(t *testing.T) {
    assert(true)
}
";
    let parser = GoParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "init");
    assert_eq!(functions[1].name, "main");
    assert_eq!(functions[2].name, "TestSomething");
}

#[test]
fn test_go_parser_multiple_returns() {
    let content = r#"
func divide(a, b int) (int, error) {
    if b == 0 {
        return 0, errors.New("division by zero")
    }
    return a / b, nil
}

func namedReturns(x int) (result int, err error) {
    result = x * 2
    return
}
"#;
    let parser = GoParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "divide");
    assert_eq!(functions[1].name, "namedReturns");
}

#[test]
fn test_go_parser_receiver_types() {
    let content = r"
func (u User) String() string {
    return u.Name
}

func (u *User) SetName(name string) {
    u.Name = name
}

func (s MySlice) Len() int {
    return len(s)
}
";
    let parser = GoParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "String");
    assert_eq!(functions[1].name, "SetName");
    assert_eq!(functions[2].name, "Len");
}

#[test]
fn test_go_parser_variadic() {
    let content = r"
func sum(nums ...int) int {
    total := 0
    for _, n := range nums {
        total += n
    }
    return total
}

func printf(format string, args ...interface{}) {
    fmt.Printf(format, args...)
}
";
    let parser = GoParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "sum");
    assert_eq!(functions[1].name, "printf");
}
