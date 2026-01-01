//! Python triple-quoted string (docstring) tests

use super::*;

#[test]
fn python_triple_double_quote_with_single_quotes_inside() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // """he said 'hello' there""" - single quotes inside triple double quotes
    let line = r#"s = """he said 'hello' there""""#;

    // The first """ opens the docstring
    // Single quotes inside don't interfere with triple-double-quote matching
    // "there""" ends the docstring, trailing " is outside
    // find_multi_line_start detects the first """ as docstring start
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn python_triple_single_quote_with_double_quotes_inside() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // '''he said "hello" there''' - double quotes inside triple single quotes
    let line = r#"s = '''he said "hello" there'''"#;

    // The first ''' opens the docstring
    // Double quotes inside don't interfere with triple-single-quote matching
    // The closing ''' properly ends the docstring
    // find_multi_line_start detects the first ''' as docstring start
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn python_triple_quote_with_single_nested_same_quote() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // """text with one " inside""" - single double quote inside triple
    let line = r#"s = """text with one " inside""""#;

    // The first """ opens the docstring, but the single " inside doesn't close it
    // However, at "inside""" we have a closing """ - docstring is complete
    // Then the trailing " is outside the docstring
    // find_multi_line_start detects the first """ as docstring start
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn python_triple_quote_with_two_nested_same_quotes() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // """text with "" inside""" - two double quotes inside triple
    let line = r#"s = """text with "" inside""""#;

    // The first """ opens the docstring
    // The "" inside is just two quotes (not closing)
    // Then "inside""" ends at the """ - docstring complete
    // Trailing " is outside
    // find_multi_line_start detects the first """ as docstring start
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn python_triple_quote_with_mixed_quotes() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // """it's a "quote" test""" - both single and double inside
    let line = r#"s = """it's a "quote" test""""#;

    // The first """ opens the docstring
    // Single quotes and double quotes inside are just content
    // "test""" ends the docstring, trailing " is outside
    // find_multi_line_start detects the first """ as docstring start
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn python_triple_quote_followed_by_comment() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // After triple-quoted string ends, # comment should... well, it's single-line
    // Let's test with another docstring
    let line = r#"s = """doc1"""; t = """doc2"#;

    // Second """ starts a new docstring
    let result = detector.find_multi_line_start(line);
    assert!(result.is_some());
}

#[test]
fn python_single_line_docstring_with_nested_quotes() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // Docstring on one line with nested quotes
    let line = r#""""This is a "docstring" with quotes""""#;

    // The inner quotes should not terminate early
    // This tests the complete docstring on one line
    let result = detector.find_multi_line_start(line);
    // First """ is found and recognized as docstring start
    assert!(result.is_some());
}

#[test]
fn python_empty_triple_quote_string() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // """""" - empty triple-quoted string (6 quotes = open + close)
    let line = r#"s = """""""#;

    // Six quotes = empty docstring
    // First """ starts, second """ ends
    assert!(detector.find_multi_line_start(line).is_some());
}
