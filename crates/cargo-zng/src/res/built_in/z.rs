use super::*;

const Z_HELP: &str = r#"
Skip an entire pass for each 'z'

These request files:
  source/echo 'sh
   | echo "first pass"
  source/echo 'z'sh
   | echo "second pass"
  source/echo 'zz'sh
   | echo "third pass"
  source/other 'rp'warn
   | other, ${ZR_APP}!

Will print in this order:

| first pass
| other, Foo
| second pass
| third pass

Note that the recursive tool 'rp'warn runs before the second pass too, because
recursive tools execute immediately.

The 'z tool exists to enable packages to stage files for a final tool to aggregate the result

The 'zz or 'zzz... "tools" are a special syntax, 'zz'tool is the equivalent of 'z'z'tool
"#;
pub(super) fn z() {
    help(Z_HELP);
}
