use std::{io::Write as _, mem};

use convert_case::{Case, Casing as _};

use super::*;

const RP_HELP: &str = r#"
Replace ${VAR|<file|!cmd} occurrences in the content

The request file:
  source/greetings.txt.zr-rp
   | Thanks for using ${ZR_APP}!

Writes the text content with ZR_APP replaced:
  target/greetings.txt
  | Thanks for using Foo App!

The parameters syntax is ${VAR|!|<[:[case]][?else]}:

${VAR}          — Replaces with the env var value, or fails if it is not set.
${VAR:case}     — Replaces with the env var value, case converted.
${VAR:?else}    — If VAR is not set or is empty uses 'else' instead.

${<file.txt}    — Replaces with the 'file.txt' content. 
                  Paths are relative to the workspace root.
${<file:case}   — Replaces with the 'file.txt' content, case converted.
${<file:?else}  — If file cannot be read or is empty uses 'else' instead.

${!cmd -h}      — Replaces with the stdout of the bash script line. 
                  The script runs the same bash used by '.zr-sh'.
                  The script must be defined all in one line.
                  A separate bash instance is used for each occurrence.
                  The working directory is the workspace root.
${!cmd:case}    — Replaces with the stdout, case converted. 
                  If the script contains ':' quote it with double quotes\"
${!cmd:?else}  — If script fails or ha no stdout, uses 'else' instead.

$${VAR}         — Escapes $, replaces with '${VAR}'.

The :case functions are:

:k or :kebab  — kebab-case (cleaned)
:K or :KEBAB  — UPPER-KEBAB-CASE (cleaned)
:s or :snake  — snake_case (cleaned)
:S or :SNAKE  — UPPER_SNAKE_CASE (cleaned)
:l or :lower  — lower case
:U or :UPPER  — UPPER CASE
:T or :Title  — Title Case
:c or :camel  — camelCase (cleaned)
:P or :Pascal — PascalCase (cleaned)
:Tr or :Train — Train-Case (cleaned)
:           — Unchanged
:clean      — Cleaned
:f or :file — Sanitize file name

Cleaned values only keep ascii alphabetic first char and ascii alphanumerics, ' ', '-' and '_' other chars.
More then one case function can be used, separated by pipe ':T|f' converts to title case and sanitize for file name. 


The fallback(:?else) can have nested ${...} patterns. 
You can set both case and else: '${VAR:case?else}'.

Variables:

All env variables can be used, of particular use with this tool are:

ZR_APP_ID — package.metadata.zng.about.app_id or "qualifier.org.app" in snake_case
ZR_APP — package.metadata.zng.about.app or package.name
ZR_ORG — package.metadata.zng.about.org or the first package.authors
ZR_VERSION — package.version
ZR_DESCRIPTION — package.description
ZR_HOMEPAGE — package.homepage
ZR_LICENSE — package.license
ZR_PKG_NAME — package.name
ZR_PKG_AUTHORS — package.authors
ZR_CRATE_NAME — package.name in snake_case
ZR_QUALIFIER — package.metadata.zng.about.qualifier or the first components `ZR_APP_ID` except the last two
ZR_META_*` — any other custom string value in package.metadata.zng.about.*

See `zng::env::about` for more details about metadata vars.
See the cargo-zng crate docs for a full list of ZR vars.

"#;
pub(super) fn rp() {
    help(RP_HELP);

    // target derived from the request place
    let content = fs::File::open(path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("cannot read, {e}"));
    let target = path(ZR_TARGET);
    let target = fs::File::create(target).unwrap_or_else(|e| fatal!("cannot write, {e}"));
    let mut target = io::BufWriter::new(target);

    let mut content = io::BufReader::new(content);
    let mut line = String::new();
    let mut ln = 1;
    while content.read_line(&mut line).unwrap_or_else(|e| fatal!("cannot read, {e}")) > 0 {
        let line_r = replace(&line, 0).unwrap_or_else(|e| fatal!("line {ln}, {e}"));
        target.write_all(line_r.as_bytes()).unwrap_or_else(|e| fatal!("cannot write, {e}"));
        ln += 1;
        line.clear();
    }
    target.flush().unwrap_or_else(|e| fatal!("cannot write, {e}"));
}

const MAX_RECURSION: usize = 32;
fn replace(line: &str, recursion_depth: usize) -> Result<String, String> {
    let mut n2 = '\0';
    let mut n1 = '\0';
    let mut out = String::with_capacity(line.len());

    let mut iterator = line.char_indices();
    'main: while let Some((ci, c)) = iterator.next() {
        if n1 == '$' && c == '{' {
            out.pop();
            if n2 == '$' {
                out.push('{');
                n1 = '{';
                continue 'main;
            }

            let start = ci + 1;
            let mut depth = 0;
            let mut end = usize::MAX;
            'seek_end: for (i, c) in iterator.by_ref() {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    if depth == 0 {
                        end = i;
                        break 'seek_end;
                    }
                    depth -= 1;
                }
            }
            if end == usize::MAX {
                let end = (start + 10).min(line.len());
                return Err(format!("replace not closed at: ${{{}", &line[start..end]));
            } else {
                let mut var = &line[start..end];
                let mut case = "";
                let mut fallback = None;

                // escape ":"
                let mut search_start = 0;
                if var.starts_with('!') {
                    let mut quoted = false;
                    let mut escape_next = false;
                    for (i, c) in var.char_indices() {
                        if mem::take(&mut escape_next) {
                            continue;
                        }
                        if c == '\\' {
                            escape_next = true;
                        } else if c == '"' {
                            quoted = !quoted;
                        } else if !quoted && c == ':' {
                            search_start = i;
                            break;
                        }
                    }
                }
                if let Some(i) = var[search_start..].find(':') {
                    let i = search_start + i;
                    case = &var[i + 1..];
                    var = &var[..i];
                    if let Some(i) = case.find('?') {
                        fallback = Some(&case[i + 1..]);
                        case = &case[..i];
                    }
                }

                let value = if let Some(path) = var.strip_prefix('<') {
                    match std::fs::read_to_string(path) {
                        Ok(s) => Some(s),
                        Err(e) => {
                            error!("cannot read `{path}`, {e}");
                            None
                        }
                    }
                } else if let Some(script) = var.strip_prefix('!') {
                    match sh_run(script.to_owned(), true, None) {
                        Ok(r) => Some(r),
                        Err(e) => fatal!("{e}"),
                    }
                } else {
                    env::var(var).ok()
                };

                let value = match value {
                    Some(s) => {
                        let st = s.trim();
                        if st.is_empty() {
                            None
                        } else if st == s {
                            Some(s)
                        } else {
                            Some(st.to_owned())
                        }
                    }
                    _ => None,
                };

                if let Some(mut value) = value {
                    for case in case.split('|') {
                        value = match case {
                            "k" | "kebab" => util::clean_value(&value, false).unwrap().to_case(Case::Kebab),
                            "K" | "KEBAB" => util::clean_value(&value, false).unwrap().to_case(Case::UpperKebab),
                            "s" | "snake" => util::clean_value(&value, false).unwrap().to_case(Case::Snake),
                            "S" | "SNAKE" => util::clean_value(&value, false).unwrap().to_case(Case::UpperSnake),
                            "l" | "lower" => value.to_case(Case::Lower),
                            "U" | "UPPER" => value.to_case(Case::Upper),
                            "T" | "Title" => value.to_case(Case::Title),
                            "c" | "camel" => util::clean_value(&value, false).unwrap().to_case(Case::Camel),
                            "P" | "Pascal" => util::clean_value(&value, false).unwrap().to_case(Case::Pascal),
                            "Tr" | "Train" => util::clean_value(&value, false).unwrap().to_case(Case::Train),
                            "" => value,
                            "clean" => util::clean_value(&value, false).unwrap(),
                            "f" | "file" => sanitise_file_name::sanitise(&value),
                            unknown => return Err(format!("unknown case '{unknown}'")),
                        };
                    }
                    out.push_str(&value);
                } else if let Some(fallback) = fallback {
                    if let Some(error) = fallback.strip_prefix('!') {
                        if error.contains('$') && recursion_depth < MAX_RECURSION {
                            return Err(replace(error, recursion_depth + 1).unwrap_or_else(|_| error.to_owned()));
                        } else {
                            return Err(error.to_owned());
                        }
                    } else if fallback.contains('$') && recursion_depth < MAX_RECURSION {
                        out.push_str(&replace(fallback, recursion_depth + 1)?);
                    } else {
                        out.push_str(fallback);
                    }
                } else {
                    return Err(format!("${{{var}}} output is empty"));
                }
            }
        } else {
            out.push(c);
        }
        n2 = n1;
        n1 = c;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_tests() {
        unsafe {
            // SAFETY: potentially not safe as tests run in parallel and I don't want to audit every C dep
            // of code that runs in other tests. If a segfault happen during test run caused by this I intend
            // to print the test runner log and frame it.
            std::env::set_var("ZR_RP_TEST", "test value");
        }

        assert_eq!("", replace("", 0).unwrap());
        assert_eq!("normal text", replace("normal text", 0).unwrap());
        assert_eq!("escaped ${NOT}", replace("escaped $${NOT}", 0).unwrap());
        assert_eq!("replace 'test value'", replace("replace '${ZR_RP_TEST}'", 0).unwrap());
        assert_eq!("${} output is empty", replace("empty '${}'", 0).unwrap_err()); // hmm
        assert_eq!(
            "${ZR_RP_TEST_NOT_SET} output is empty",
            replace("not set '${ZR_RP_TEST_NOT_SET}'", 0).unwrap_err()
        );
        assert_eq!(
            "not set 'fallback!'",
            replace("not set '${ZR_RP_TEST_NOT_SET:?fallback!}'", 0).unwrap()
        );
        assert_eq!(
            "not set 'nested 'test value'.'",
            replace("not set '${ZR_RP_TEST_NOT_SET:?nested '${ZR_RP_TEST}'.}'", 0).unwrap()
        );
        assert_eq!("test value", replace("${ZR_RP_TEST_NOT_SET:?${ZR_RP_TEST}}", 0).unwrap());
        assert_eq!(
            "curly test value",
            replace("curly ${ZR_RP_TEST:?{not {what} {is} {going {on {here {:?}}}}}}", 0).unwrap()
        );

        assert_eq!("replace not closed at: ${MISSING", replace("${MISSING", 0).unwrap_err());
        assert_eq!("replace not closed at: ${MIS", replace("${MIS", 0).unwrap_err());
        assert_eq!("replace not closed at: ${MIS:?{", replace("${MIS:?{", 0).unwrap_err());
        assert_eq!("replace not closed at: ${MIS:?{}", replace("${MIS:?{}", 0).unwrap_err());

        assert_eq!("TEST VALUE", replace("${ZR_RP_TEST:U}", 0).unwrap());
        assert_eq!("TEST-VALUE", replace("${ZR_RP_TEST:K}", 0).unwrap());
        assert_eq!("TEST_VALUE", replace("${ZR_RP_TEST:S}", 0).unwrap());
        assert_eq!("testValue", replace("${ZR_RP_TEST:c}", 0).unwrap());
    }

    #[test]
    fn replace_cmd_case() {
        assert_eq!("cmd HELLO:?WORLD", replace("cmd ${!printf \"hello:?world\":U}", 0).unwrap(),)
    }
}
