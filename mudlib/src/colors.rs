//! Turn codes like "`w" into "\e[37m".
//!
//! This allows writing strings like "Hello `Rworld`^", where "world" will be
//! colored bright red.

use std::{borrow::Cow, collections::HashMap};

use lazy_static::lazy_static;

static COLOR_CODES: &[(char, &str)] = &[
    ('s', "\x1b[1;30m"),
    ('S', "\x1b[1;30m"),
    ('r', "\x1b[0;31m"),
    ('g', "\x1b[0;32m"),
    ('y', "\x1b[0;33m"),
    ('b', "\x1b[0;34m"),
    ('m', "\x1b[0;35m"),
    ('c', "\x1b[0;36m"),
    ('w', "\x1b[0;37m"),
    ('R', "\x1b[1;31m"),
    ('G', "\x1b[1;32m"),
    ('Y', "\x1b[1;33m"),
    ('B', "\x1b[1;34m"),
    ('M', "\x1b[1;35m"),
    ('C', "\x1b[1;36m"),
    ('W', "\x1b[1;37m"),
    ('^', "\x1b[0m"),
    ('1', "\r\n"),
    ('N', "Demi MUD"),
];

lazy_static! {
    static ref COLOR_CODE_MAP: HashMap<char, &'static str> = {
        let mut map = HashMap::new();

        for (character, color_code) in COLOR_CODES {
            map.insert(*character, *color_code);
        }

        map
    };
}

pub fn colorize(text: &str) -> Cow<'_, str> {
    if !text.contains('`') {
        return Cow::Borrowed(text);
    };

    let mut buffer = String::new();
    let mut processed = 0;

    while let Some(backtick) = text[processed..].find('`') {
        buffer.push_str(&text[processed..processed + backtick]);
        processed += backtick;

        assert_eq!(&text[processed..processed + 1], "`");
        processed += 1;

        let color_character = if let Some(c) = text[processed..].chars().next() {
            processed += c.len_utf8();
            c
        } else {
            '^'
        };

        let color_code = COLOR_CODE_MAP.get(&color_character).unwrap_or(&"\x1b[0m");

        buffer.push_str(color_code);
    }

    buffer.push_str(&text[processed..]);

    Cow::Owned(buffer)
}

pub fn recolor<'a>(color: &str, text: &'a str) -> Cow<'a, str> {
    if text.contains('`') {
        Cow::Owned(text.replace("`^", color).replace("`x", color))
    } else {
        Cow::Borrowed(text)
    }
}
