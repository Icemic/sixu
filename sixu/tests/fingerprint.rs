use sixu::parser::parse;

#[test]
fn parsed_argument_order_is_ignored() {
    let first = parse(
        "test",
        r#"
::entry {
    @show speaker="alice" line="hello"
}
"#,
    )
    .unwrap()
    .1;
    let second = parse(
        "test",
        r#"
::entry {
    @show line="hello" speaker="alice"
}
"#,
    )
    .unwrap()
    .1;

    assert_eq!(
        first.paragraphs[0].block.fingerprint(),
        second.paragraphs[0].block.fingerprint()
    );
}

#[test]
fn parsed_block_fingerprint_matches_golden_value() {
    let story = parse(
        "test",
        r#"
::entry {
    [npc] `hello ${name}`

    @show speaker="alice" line="hello"

    {
        ##
        let score = 1;
        ##
    }
}
"#,
    )
    .unwrap()
    .1;

    assert_eq!(
        story.paragraphs[0].block.fingerprint().to_hex(),
        "dc5f9bd6bcc453d3e085da7f07b1f2ef"
    );
}
