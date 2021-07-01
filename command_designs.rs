#[widget($crate::text_box)]
pub mod text_box {
  properties! {
    on_copy = {
      enabled: |ctx| IsEnabled::get(ctx) && IsVisible::get(ctx),
      handler: hn!(|ctx| {
        let text = ctx.widget_state[TextBoxTextState];
        ctx.services.clipboard().set_text(text);
      })
    };

    on_pre_save = {
      enabled: ?
      handlr: pre_handler
    }
  }
}

context_var! {
  struct CanSaveCommandVar: bool = false;
}

struct SaveCommand;

#[property(context)]
pub fn can_save(child: impl UiNode, can_save: impl FnMut(&mut WidgetContext) -> bool) -> impl UiNode {
    can_command(child, SaveCommand, CanSaveCommandVar, can_save)
}

#[property(event)]
pub fn on_pre_save(child: impl UiNode, handler: impl WidgetHandler<CommandArgs>) -> impl UiNode {
    on_pre_command(child, SaveCommand, CanSaveCommandVar, handler)
}

#[property(event)]
pub fn on_save(child: impl UiNode, handler: impl WidgetHandler<CommandArgs>) -> impl UiNode {
    on_command(child, SaveCommand, CanSaveCommandVar, handler)
}

command_properties! {
  /// Docs 
  pub save: SaveCommand;
}

// 
// How to bind shortcuts
//

fn main() {
  App::default().run(|ctx| {
    CopyCommand::shortcuts().set(ctx.vars, shortcut!(Ctrl+C));
  })
}


//
// //
// // New Design (current sketch)
// //
//

fn button_click(ctx: &mut WidgetContext) {
  CopyCommand::notify(ctx, args);
}

fn text_box_on_event(ctx: &mut WidgetContext, args: EventUpdateArgs) {
  if let Some(args) = CopyCommand.update(args) {
    ctx.services.clipboard().set_text("selected text")
  }
}

Struct! {
  impl CopyCommand {
    fn meta -> std::thread::LocalKey
    fn label -> RcVar<Text>
    fn enabled -> ReadOnlyVar<bool>
    fn active -> ReadOnlyVar<bool>
  }  
}

button!{
  content = text(CopyCommand::label());
  enabled = CopyCommand::enabled();
  visible = CopyCommand::has_handlers();

  //this
  command = CopyCommand;
  //or this
  on_click = hn!(|_, _| CopyCommand.notify(None));
}

command_button!{
  //            content = text(CopyCommand::label());
  // default =  enabled = CopyCommand::enabled();
  //            visible = CopyCommand::has_handlers();
  command = CopyCommand;
}

fn text_box_on_init(&mut self, ctx: &mut WidgetContext) {
  self.copy_handle = CopyCommand::handle(ctx);
}

fn text_box_on_update(&mut self, ctx: &mut WidgetContext) {
  self.copy_handle.set_enabled(self.text_selected.is_some())
}
