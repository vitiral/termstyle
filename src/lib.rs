/* Copyright (c) 2018 Garrett Berg, vitiral@gmail.com
 *
 * Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
 * http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
 * http://opensource.org/licenses/MIT>, at your option. This file may not be
 * copied, modified, or distributed except according to those terms.
 */
//! termstyle: create and test the style and formatting for the text in your terminal applications
//!
//! **termstyle** is a library that aims to make it easy to build **formatted** and **styled**
//! command line applications.
//!
//! # Examples
//! ```rust
//! # extern crate termstyle;
//! extern crate serde_yaml;
//! use termstyle::{Color, El, Text};
//!
//! # fn main() {
//! // example raw yaml file
//! let example = r#"
//! - ["plain ", {t: "and bold", b: true}]
//! - ["plain ", {t: "and red", c: red}]
//! "#;
//!
//! // deserialize as yaml (you can use `serde_json::from_str` for json, etc).
//! let els = termstyle::from_str(serde_yaml::from_str, example).unwrap();
//!
//! // This could have also been programmatically constructed like this
//! let expected = vec![
//!     El::plain("plain ".into()),
//!     El::Text(Text::new("and bold".into()).bold()),
//!     El::plain("plain ".into()),
//!     El::Text(Text::new("and red".into()).color(Color::Red)),
//! ];
//!
//! assert_eq!(els, expected);
//!
//! // Render directly to stdout. Can also use `El::paint` to
//! // do one at a time.
//! termstyle::paint(&mut ::std::io::stdout(), &els).unwrap();
//! # }
//! ```
//!
//! # Styled Tables
//! See the documentation for [`Table`](struct.Table.html)

extern crate ansi_term;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate std_prelude;
extern crate tabwriter;

use std::io;
use std_prelude::*;
use ansi_term::Color as AColor;

/// Convert a string into `Vec<El>` using the given deserializer.
pub fn from_str<E, F>(_from_str: F, s: &str) -> Result<Vec<El>, E>
where
    F: Fn(&str) -> Result<Vec<ElRaw>, E>,
{
    let raw: Vec<ElRaw> = _from_str(s)?;
    let mut out: Vec<El> = Vec::new();
    flatten_raw(&mut out, raw);
    Ok(out)
}

/// Paint the given elements into the writer.
///
/// Useful after loading them with `from_str`. Can also be useful if you build your elements as a
/// `Vec` and want to paint them all in one go.
///
/// Alternatively, try calling the `paint()` method on the types themselves, (i.e. `El::paint()`).
pub fn paint<W: io::Write>(w: &mut W, items: &[El]) -> io::Result<()> {
    for item in items {
        item.paint(w)?;
    }
    Ok(())
}

/// Helper function to make tests easier for others.
///
/// If a diff exists, render the full form of both and their "repr" version to stderr, then return
/// their human readable and copy-pastable renderings.
///
/// This is useful for testing, as you can clearly see the differences.
///
/// It is recommended to then `assert_eq` on the response using the excelent `pretty_assertiosn`
/// crate to help you diagnose any issues.
///
/// # Examples
/// ```
/// # extern crate termstyle;
/// # fn main() {
/// let result = vec![55, 60, 73, 7, 145, 80];
/// let expected = vec![55, 60, 73, 7, 145, 80];
/// let (repr_result, repr_expected) = termstyle::eprint_diff(&expected, &result);
/// assert_eq!(repr_result, repr_expected);
/// # }
/// ```
///
pub fn eprint_diff(expected: &[u8], result: &[u8]) -> (String, String) {
    let mut expected_repr: Vec<u8> = Vec::new();
    write_repr(&mut expected_repr, expected).unwrap();

    let mut result_repr: Vec<u8> = Vec::new();
    write_repr(&mut result_repr, result).unwrap();

    if result == expected {
        return (
            String::from_utf8(expected_repr).unwrap(),
            String::from_utf8(result_repr).unwrap(),
        );
    }

    eprintln!("Bytes are not equal");
    eprintln!("## EXPECTED");
    {
        io::stderr().write_all(expected).unwrap();
    }
    eprintln!("\n# RAW-EXPECTED");
    eprint_repr(expected);

    eprintln!("\n\n## RESULT");
    {
        io::stderr().write_all(result).unwrap();
    }
    eprintln!("\n# RAW-RESULT");
    eprint_repr(result);
    eprintln!();

    (
        String::from_utf8(expected_repr).unwrap(),
        String::from_utf8(result_repr).unwrap(),
    )
}

/// Helper function to make tests easier for others.
///
/// Represent a series of bytes with proper escape codes for
/// copy/pasting into rust surounded by `b"..."`.
pub fn write_repr<W: io::Write>(w: &mut W, bytes: &[u8]) -> io::Result<()> {
    for b in bytes {
        match *b {
            b'\t' => write!(w, r"\t")?,
            b'\n' => write!(w, r"\n")?,
            b'\r' => write!(w, r"\r")?,
            b'\\' => write!(w, r"\\")?,
            32...126 => write!(w, "{}", *b as char)?, // visible ASCII
            _ => write!(w, r"\x{:0>2x}", b)?,
        }
    }
    Ok(())
}

#[test]
fn sanity_repr() {
    let r = |b| {
        let mut w: Vec<u8> = Vec::new();
        write_repr(&mut w, b).unwrap();
        String::from_utf8(w).unwrap()
    };
    assert_eq!(r"foo", r(b"foo"));
    assert_eq!(r"\\", r(b"\\"));
    assert_eq!(r"\x8a", r(b"\x8a"));
    assert_eq!(r"\x8a", r(&[0x8a]));
}

/// Helper function to make tests easier for others.
///
/// Print the byte representation directly to stdout.
///
/// See [`repr`](fn.repr.html)
pub fn print_repr(bytes: &[u8]) {
    write_repr(&mut io::stdout(), bytes).expect("print_repr");
}

/// Print the byte representation directly to stderr.
///
/// See [`repr`](fn.repr.html)
pub fn eprint_repr(bytes: &[u8]) {
    write_repr(&mut io::stderr(), bytes).expect("eprint_repr");
}

#[derive(Debug, Eq, PartialEq)]
/// A Element that can be rendered as styled+formatted text using `paint()`.
///
/// Elements are simply struts with various properties which you can build directly or parse from
/// text.
pub enum El {
    Text(Text),
    Table(Table),
}

#[derive(Debug, Eq, PartialEq)]
/// A paintable Table
///
/// The type can be thought of as `Rows[Cols[Cells[Text]]]`, where the items inside a `Cell`
/// will be concatenated together (alowing mixed formatting to exist within a table's cell).
///
/// Warning: do not use `\t` in your text, as this currently uses tabwriter under the hood.
///
/// # Examples
/// ```rust
/// # extern crate termstyle;
/// use termstyle::*;
///
/// # fn main() {
/// let rows = vec![
///     // header
///     vec![
///         vec![Text::new("header1".into())],
///         vec![Text::new("header2".into())],
///     ],
///     // row1
///     vec![
///         vec![Text::new("col1".into())],
///         vec![Text::new("col2".into())],
///     ],
/// ];
/// let example = Table::new(rows);
///
/// let expected = "\
/// header1 header2
/// col1    col2
/// ";
///
/// let mut result = Vec::new();
/// example.paint(&mut result);
///
/// assert_eq!(expected.as_bytes(), result.as_slice());
/// # }
/// ```
pub struct Table {
    table: Vec<Vec<Vec<Text>>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// Possible Terminal Colors
pub enum Color {
    Plain,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Purple,
    Cyan,
    White,
    // TODO: non-trivial in serde
    // Fixed(u8),
    // RGB(u8, u8, u8),
}

#[derive(Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
/// A piece of text, may be colored, etc
pub struct Text {
    t: String, // 'text'
    b: bool,   // 'bold'
    i: bool,   // 'italic'
    c: Color,  // 'color'
    bg: Color, // 'background color'
}

impl Default for Color {
    fn default() -> Color {
        Color::Plain
    }
}

impl Color {
    fn to_ansi(&self) -> Option<AColor> {
        match *self {
            Color::Plain => None,
            Color::Black => Some(AColor::Black),
            Color::Red => Some(AColor::Red),
            Color::Green => Some(AColor::Green),
            Color::Yellow => Some(AColor::Yellow),
            Color::Blue => Some(AColor::Blue),
            Color::Purple => Some(AColor::Purple),
            Color::Cyan => Some(AColor::Cyan),
            Color::White => Some(AColor::White),
            // TODO: It seems that serde cannot handle this easily
            // Color::Fixed(a)          =>   Some(AColor::Fixed(a)),
            // Color::RGB(a, b, c)      =>   Some(AColor::RGB(a, b, c)),
        }
    }
}

impl El {
    /// Instantiate the element as just plain text.
    ///
    /// This is a shortcut method for `El::Text(Text::new(t))`. In general you should use
    /// `El::Text` or `El::Table` instead.
    pub fn plain(t: String) -> El {
        El::Text(Text::new(t))
    }

    /// Recursively clears _all_ formatting.
    pub fn set_plain(&mut self) {
        match *self {
            El::Text(ref mut t) => t.set_plain(),
            El::Table(ref mut t) => t.set_plain(),
        }
    }

    /// Paint (render) the item into the writer.
    pub fn paint<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
        match *self {
            El::Text(ref t) => t.paint(w),
            El::Table(ref t) => t.paint(w),
        }
    }
}

impl Table {
    /// Create a new table from the given rows.
    ///
    /// The type can be thought of as `Rows[Cols[Cells[Text]]]`, where the items inside a `Cell`
    /// will be concatenated together (alowing mixed formatting to exist within a table's cell).
    pub fn new(table: Vec<Vec<Vec<Text>>>) -> Table {
        Table { table: table }
    }

    /// Recursively clears _all_ formatting.
    pub fn set_plain(&mut self) {
        for row in &mut self.table {
            for col in row {
                for t in col {
                    t.set_plain();
                }
            }
        }
    }

    /// Paint the table, giving each column the same width.
    pub fn paint<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
        // println!("Painting table:\n{:#?}\n", self);
        let mut tw = tabwriter::TabWriter::new(Vec::new()).padding(1);
        for row in &self.table {
            for (i, cell) in row.iter().enumerate() {
                for text in cell {
                    text.paint(&mut tw)?;
                }
                if i < row.len() - 1 {
                    write!(&mut tw, "\t")?;
                }
            }
            write!(&mut tw, "\n")?;
        }
        tw.flush()?;
        w.write_all(&tw.into_inner().unwrap())
    }
}

impl Text {
    /// Instantiate the Text as just plain text.
    ///
    /// Use the builder pattern to construct the rest.
    ///
    /// # Examples
    /// ```rust
    /// # extern crate termstyle;
    /// use termstyle::{Color, Text};
    ///
    /// # fn main() {
    /// let t = Text::new("bold and blue text".into())
    ///     .bold()
    ///     .color(Color::Blue);
    ///
    /// // write it to stdout
    /// t.paint(&mut ::std::io::stdout()).unwrap();
    /// # }
    /// ```
    pub fn new(t: String) -> Text {
        Text {
            t: t,
            b: false,
            i: false,
            c: Color::default(),
            bg: Color::default(),
        }
    }

    /// Make the text styled as bold
    pub fn bold(mut self) -> Text {
        self.b = true;
        self
    }

    /// Make the text styled as italic
    pub fn italic(mut self) -> Text {
        self.i = true;
        self
    }

    /// Set the color style of the text
    pub fn color(mut self, color: Color) -> Text {
        self.c = color;
        self
    }

    #[cfg(unix)]
    fn style(&self) -> ansi_term::Style {
        let mut style = ansi_term::Style::new();
        if self.b {
            style = style.bold();
        }
        if self.i {
            style = style.italic();
        }
        style = match self.c.to_ansi() {
            None => style,
            Some(c) => style.fg(c),
        };

        style = match self.bg.to_ansi() {
            None => style,
            Some(c) => style.on(c),
        };
        style
    }

    #[cfg(not(unix))]
    fn style(&self) -> ansi_term::Style {
        // TODO: no style for non-unix systems
        ansi_term::Style::new()
    }

    pub fn paint<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
        let style = self.style();
        write!(w, "{}", style.paint(self.t.as_str()))
    }

    pub fn is_bold(&self) -> bool {
        self.b
    }

    pub fn is_italic(&self) -> bool {
        self.i
    }

    pub fn is_plain(&self) -> bool {
        self.c == Color::Plain
    }

    pub fn get_color(&self) -> Color {
        self.c
    }

    /// Clears _all_ formatting.
    pub fn set_plain(&mut self) {
        self.b = false;
        self.i = false;
        self.c = Color::Plain;
        self.bg = Color::Plain;
    }
}

// PRIVATE: priate types and methods

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Raw `El` type, used only for deserializing.
pub enum ElRaw {
    // This MUST be first
    Table(TableRaw),
    Text(TextsRaw),
}

#[derive(Debug, Serialize, Deserialize)]
/// Raw `Table` type, used only for deserializing.
pub struct TableRaw {
    table: Vec<Vec<TextsRaw>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Raw `Text` type, used so you can specify `"foo bar"` or `["foo ", "bar"]`
pub enum TextsRaw {
    Single(TextRaw),
    Multi(Vec<TextRaw>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Raw `Text` type, used only for deserializing.
pub enum TextRaw {
    Simple(String),
    Full(Text),
}

fn flatten_raw(into: &mut Vec<El>, mut raw: Vec<ElRaw>) {
    for el in raw.drain(..) {
        flatten_el(into, el);
    }
}

fn flatten_el(into: &mut Vec<El>, raw: ElRaw) {
    match raw {
        ElRaw::Text(t) => flatten_texts(into, t),
        ElRaw::Table(mut table_raw) => {
            let mut table = Vec::new();
            for mut row_raw in table_raw.table.drain(..) {
                let mut row = Vec::new();
                for cell_raw in row_raw.drain(..) {
                    let mut cell = Vec::new();
                    flatten_texts_only(&mut cell, cell_raw);
                    row.push(cell);
                }
                table.push(row);
            }
            into.push(El::Table(Table { table: table }));
        }
    }
}

fn flatten_texts(into: &mut Vec<El>, raw: TextsRaw) {
    match raw {
        TextsRaw::Single(t) => into.push(El::Text(Text::from(t))),
        TextsRaw::Multi(mut multi) => into.extend(multi.drain(..).map(|t| El::Text(Text::from(t)))),
    }
}

fn flatten_texts_only(into: &mut Vec<Text>, raw: TextsRaw) {
    match raw {
        TextsRaw::Single(t) => into.push(Text::from(t)),
        TextsRaw::Multi(mut multi) => into.extend(multi.drain(..).map(Text::from)),
    }
}

impl From<TextRaw> for Text {
    fn from(raw: TextRaw) -> Text {
        match raw {
            TextRaw::Simple(t) => Text::new(t),
            TextRaw::Full(f) => f,
        }
    }
}
