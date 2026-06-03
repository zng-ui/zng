This is a `cargo zng res` package used by `build.rs` to collect and optimize the embedded localization resources example.

To update optimization profile run:

ZNG_L10N_PROFILE_FILE=examples/localize/res/l10n/usage.rec.subset cargo do run localize --features l10n_usage_recorder
