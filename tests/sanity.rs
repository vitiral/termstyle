/* Copyright (c) 2017 Garrett Berg, vitiral@gmail.com
 *
 * Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
 * http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
 * http://opensource.org/licenses/MIT>, at your option. This file may not be
 * copied, modified, or distributed except according to those terms.
 */
//! Sanity tests

#[macro_use]
extern crate pretty_assertions;

extern crate serde_json;
extern crate serde_yaml;
extern crate termstyle;

use termstyle::*;

static BASIC_YAML: &str = r#"
- some plain text
- {t: " bold text ", b:true}
- {t: red-only, c:red}
- {t: " green-only\n", c:green}
-
    t: |-
        defined in multiple lines with multiple things
        is multiple lines
    i: true
    b: true
    c: green
- ["\nall in ", {t: "one line!!!", b: true}]
"#;

pub fn from_yaml(s: &str) -> Vec<El> {
    from_str(serde_yaml::from_str, s).unwrap()
}

#[test]
fn sanity_deserialize() {
    let items = from_yaml(BASIC_YAML);
    let expected = vec![
        El::plain("some plain text".into()),
        El::Text(Text::new(" bold text ".into()).bold()),
        El::Text(Text::new("red-only".into()).color(Color::Red)),
        El::Text(Text::new(" green-only\n".into()).color(Color::Green)),
        El::Text(
            Text::new(
                "\
                 defined in multiple lines with multiple things\n\
                 is multiple lines"
                    .into(),
            ).italic()
                .bold()
                .color(Color::Green),
        ),
        El::Text(Text::new("\nall in ".into())),
        El::Text(Text::new("one line!!!".into()).bold()),
    ];
    assert_eq!(items, expected);
}

#[cfg(unix)]
static BASIC_YAML_RENDERED: &[u8] = b"\
some plain text\x1b[1m bold text \x1b[0m\x1b[31mred-only\x1b[0m\x1b[32m green-only\n\
\x1b[0m\x1b[1;3;32mdefined in multiple lines with multiple things\n\
is multiple lines\x1b[0m\n\
all in \\x1b[1mone line!!!\\x1b[0m\
";

#[cfg(not(unix))]
static BASIC_YAML_RENDERED: &[u8] = b"\
some plain text bold text red-only green-only\n\
defined in multiple lines with multiple things\n\
is multiple lines\n\
all in one line!!!\
";

#[test]
fn sanity_paint() {
    let items = from_yaml(BASIC_YAML);
    let mut result: Vec<u8> = Vec::new();
    paint(&mut result, &items).unwrap();
    let (repr_e, repr_r) = eprint_diff(BASIC_YAML_RENDERED, &result);
    assert_eq!(repr_e, repr_r);
}

#[test]
fn sanity_table() {
    let yaml_raw = r#"
- table:
  - ["header1", "header2"]
  - ["col1", "col2"]
"#;
    let elements = from_yaml(yaml_raw);
    let expected = vec![
        El::Table(Table::new(vec![
            // header
            vec![
                vec![Text::new("header1".into())],
                vec![Text::new("header2".into())],
            ],
            // row1
            vec![
                vec![Text::new("col1".into())],
                vec![Text::new("col2".into())],
            ],
        ])),
    ];
    assert_eq!(expected, elements);

    let mut result: Vec<u8> = Vec::new();
    paint(&mut result, &elements).unwrap();
    let expected = b"header1 header2\ncol1    col2\n";
    let (repr_e, repr_r) = eprint_diff(expected, &result);
    assert_eq!(repr_e, repr_r);
}

#[cfg(unix)]
static SANITY_README_RENDERED: &[u8] = b"\
\x1b[1m-- EXAMPLE --\n\
\x1b[0mThis is a regular string with a newline\n\
This does not have a newline, but \x1b[31mthis is red\x1b[0m, but this is NOT red!\n\
Bold is easy like this: \x1b[1msee I'm bold!!\n\
\x1b[0mAnd so is multiple settings\x1b[1;32m\n\
bold AND green!\n\
and even multiple lines :) :)\n\
\x1b[0m\nyou can group multiple text items \x1b[1mon one line!\x1b[0m\n\
Grouping things in one line is necessary for tables\n\
Notice that some cells are grouped and some are not.\n\n\
\x1b[1m# Table\x1b[0m\n\
header \x1b[1mcol1\x1b[0m | header col2\n\
row col1    | \x1b[32mrow col2\x1b[0m\n\
";

#[cfg(not(unix))]
static SANITY_README_RENDERED: &[u8] = b"\
-- EXAMPLE --\n\
This is a regular string with a newline\n\
This does not have a newline, but this is red, but this is NOT red!\n\
Bold is easy like this: see I'm bold!!\n\
And so is multiple settings\n\
bold AND green!\n\
and even multiple lines :) :)\n\
\nyou can group multiple text items on one line!\n\
Grouping things in one line is necessary for tables\n\
Notice that some cells are grouped and some are not.\n\n\
# Table\n\
header col1 | header col2\n\
row col1    | row col2\n\
";

#[test]
/// run a test against the example in the readme
fn sanity_readme() {
    let readme = r####"
- {t: "-- EXAMPLE --\n", b: true}
- "This is a regular string with a newline\n"
- "This does not have a newline, but "
- {t: "this is red", c: red}
- ", but this is NOT red!\n"
- "Bold is easy like this: "
- {t: "see I'm bold!!\n", b: true}
- And so is multiple settings
- # long-form
  t: |

      bold AND green!
      and even multiple lines :) :)
  b: true
  c: green
- ["\nyou can group multiple text items ", {t: "on one line!", b: true}]
- "\nGrouping things in one line is necessary for tables\n"
- "Notice that some cells are grouped and some are not.\n\n"
- [{t: "# Table", b: true}, "\n"]
-
  table:
  - [["header ", {t: "col1", b: true}] ,"| header col2"]
  - ["row col1", ["| ", {t: "row col2", c: green}]]
"####;
    let items = from_yaml(readme);
    let mut result: Vec<u8> = Vec::new();
    paint(&mut result, &items).unwrap();
    let (repr_e, repr_r) = eprint_diff(SANITY_README_RENDERED, &result);
    assert_eq!(repr_e, repr_r);
}

#[test]
fn sanity_color() {
    let plain = from_yaml("- color");
    let black = from_yaml("- {t: color, c: black}");
    let red = from_yaml("- {t: color, c: red}");
    let green = from_yaml("- {t: color, c: green}");
    let yellow = from_yaml("- {t: color, c: yellow}");
    let blue = from_yaml("- {t: color, c: blue}");
    let purple = from_yaml("- {t: color, c: purple}");
    let cyan = from_yaml("- {t: color, c: cyan}");
    let white = from_yaml("- {t: color, c: white}");
    // TODO: non-trivial in serde
    // let fixed1 = from_yaml("- {t: color, c: 10}");
    // let fixed2 = from_yaml("- {t: color, c: 100}");
    // let rgb = from_yaml("- {t: color, c: [1, 2, 3]}");

    fn assert_color(els: &[El], expected: Color) {
        let t = match els[0] {
            El::Text(ref t) => t,
            _ => panic!(),
        };

        assert_eq!(expected, t.get_color());
    }
    assert_color(&plain, Color::Plain);
    assert_color(&black, Color::Black);
    assert_color(&red, Color::Red);
    assert_color(&green, Color::Green);
    assert_color(&yellow, Color::Yellow);
    assert_color(&blue, Color::Blue);
    assert_color(&purple, Color::Purple);
    assert_color(&cyan, Color::Cyan);
    assert_color(&white, Color::White);
}
