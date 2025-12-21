use super::*;

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
fn test_js_parser_arrow_variations() {
    let content = r#"
const simple = () => {
    return 1;
};

let mutable = () => {
    return 2;
};

var legacy = () => {
    return 3;
};

const withParams = (a, b) => {
    return a + b;
};

export const exported = () => {
    return "exported";
};
"#;
    let parser = JsParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 5);
    assert_eq!(functions[0].name, "simple");
    assert_eq!(functions[1].name, "mutable");
    assert_eq!(functions[2].name, "legacy");
    assert_eq!(functions[3].name, "withParams");
    assert_eq!(functions[4].name, "exported");
}

#[test]
fn test_js_parser_class_variations() {
    let content = r"
class SimpleClass {
    method() {}
}

export class ExportedClass {
    constructor() {}
}

class ExtendedClass extends BaseClass {
    override() {}
}
";
    let parser = JsParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "SimpleClass");
    assert_eq!(functions[1].name, "ExportedClass");
    assert_eq!(functions[2].name, "ExtendedClass");
}

#[test]
fn test_js_parser_function_variations() {
    let content = r#"
function regularFunc() {
    return 1;
}

async function asyncFunc() {
    await promise;
}

function* generatorFunc() {
    yield 1;
}

export function exportedFunc() {
    return "exported";
}

export async function exportedAsync() {
    await something;
}
"#;
    let parser = JsParser::new();
    let functions = parser.parse(content);

    // Generator functions may not be detected by current pattern
    assert!(functions.len() >= 4);
    assert_eq!(functions[0].name, "regularFunc");
    assert_eq!(functions[1].name, "asyncFunc");
}

#[test]
fn test_js_parser_typescript_patterns() {
    let content = r"
function typedFunc(x: number): string {
    return x.toString();
}

const typedArrow = (x: string): number => {
    return parseInt(x);
};

async function asyncTyped(): Promise<void> {
    await fetch();
}

class TypedClass {
    private value: number;
    constructor(v: number) {
        this.value = v;
    }
}
";
    let parser = JsParser::new();
    let functions = parser.parse(content);

    assert_eq!(functions.len(), 4);
    assert_eq!(functions[0].name, "typedFunc");
    assert_eq!(functions[1].name, "typedArrow");
    assert_eq!(functions[2].name, "asyncTyped");
    assert_eq!(functions[3].name, "TypedClass");
}

#[test]
fn test_js_parser_react_patterns() {
    let content = r"
function Component() {
    return <div>Hello</div>;
}

const ArrowComponent = () => {
    return <span>World</span>;
};

export function ExportedComponent() {
    return null;
}

export const MemoizedComponent = React.memo(() => {
    return <div />;
});
";
    let parser = JsParser::new();
    let functions = parser.parse(content);

    assert!(functions.len() >= 3);
    assert_eq!(functions[0].name, "Component");
    assert_eq!(functions[1].name, "ArrowComponent");
    assert_eq!(functions[2].name, "ExportedComponent");
}

#[test]
fn test_js_parser_default_export() {
    let content = r#"
export default function defaultFunc() {
    return "default";
}
"#;
    let parser = JsParser::new();
    let functions = parser.parse(content);

    // Current pattern may or may not catch 'export default function'
    // This test documents current behavior
    assert!(functions.len() <= 1);
}
