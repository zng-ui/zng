* Implement config source combinator.
    - OverrideSource, to support a "workspace" over "user" over "defaults" type of setup.
    - SeparateSource, to support redirecting keys to different sources.

```
trait ConfigSource {
    fn with_fallback(self, other: C) -> TODO { }
    fn with_redirect(self, other: C, redirect: impl FnMut(&ConfigKey) -> bool) -> TODO { }
}
```