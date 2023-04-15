fn main() {
    let _scope = lib_detached::App::minimal();
    // uses zero_ui_core::widget_new!.
    let _wgt = lib_detached::Foo!();
}
