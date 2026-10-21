//! The playable races, for the page's character window: `cargo run --release -p sim --example
//! races [assets] > dist/races.json` (build-web.sh). For each race: its index in the rules, the
//! sexes it allows, whether its look is a whole sheet, its portrait, and its look's files for
//! each sex (keys as `Files` lists them, one list per category); its name (male and female)
//! and description in each language of `Langues.ini`, by suffix. Also the random names, male
//! and female.

use sim::files::Files;
use sim::rules::{Rules, languages};
use std::fmt::Write;
use std::path::Path;

/// `s` as a JSON string.
fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => write!(out, "\\u{:04x}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn list<T>(items: impl IntoIterator<Item = T>, f: impl Fn(T) -> String) -> String {
    format!("[{}]", items.into_iter().map(f).collect::<Vec<_>>().join(","))
}

fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets").display().to_string());
    let files = Files::from_dir(Path::new(&root)).expect("converted assets");
    let langs: Vec<(String, Rules)> = languages(&files)
        .into_iter()
        .filter_map(|l| Rules::load(&files, &l.folder).ok().map(|r| (l.suffix, r)))
        .collect();
    let (_, rules) = &langs[0];
    let races = (0..rules.races.len()).filter(|&i| rules.races[i].playable).map(|i| {
        let r = &rules.races[i];
        let by_lang = |f: &dyn Fn(&Rules) -> String| list(&langs, |(suffix, rules)| format!("{}:{}", quote(suffix), f(rules)));
        let names = by_lang(&|rules| list(&rules.races[i].name, |n| quote(n))).replacen('[', "{", 1);
        let descriptions = by_lang(&|rules| quote(&rules.races[i].description)).replacen('[', "{", 1);
        let looks = list(&r.look_files, |lists| list(lists.iter().filter(|l| !l.is_empty()), |l| list(l, |f| quote(f))));
        format!(
            "{{\"index\":{i},\"female\":{},\"male\":{},\"sheet\":{},\"portrait\":{},\"name\":{}}},\"description\":{}}},\"looks\":{looks}}}",
            !r.no_female,
            !r.no_male,
            r.whole_sheet,
            quote(&r.portrait.replace('\\', "/")),
            &names[..names.len() - 1],
            &descriptions[..descriptions.len() - 1],
        )
    });
    let names = list(&rules.names, |names| list(names, |n| quote(n)));
    println!("{{\"races\":[{}],\"names\":{names}}}", races.collect::<Vec<_>>().join(","));
}
