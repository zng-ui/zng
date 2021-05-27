// 
// How to declare
//
mod declare_stuff{
  trait Command: 'static {
    fn enabled() -> Var<bool> {}
    fn visible() -> Var<bool> {}
    fn text() -> Var<Text> {}
    fn shortcuts() -> Var<Shortcuts> {}
  }

  struct Commands {}
  impl Commands { // Sender
    fn execute<C: Command>(&mut self, command: C) {
      // emits command event
    }

    fn on_shortcut_collision(handler: Fn(MultipleCommandsSameShortcut)) {

    }
  }  
  impl Commands { // Receiver
    fn handler<C: Command>(&mut self, command: C) -> Rc<bool> {
      // keep weak ref returns strong
    }
  }

  command! {
      /// Command docs.
      pub CopyCommand;

      /// Other command docs.
      pub PasteCommand;
  }

}

// 
// How to use (Send)
//
mod send_stuff {
  button!{
    command = CopyCommand;
    //enabled = CopyCommand::enabled();
    //visibility = CopyCommand::visible().map(Visibility::from);
    content = text(CopyCommand::text());
  }

  #[property(context)]
  fn command<C: Command>(child: impl UiNode, command: C) -> impl UiNode {
    let node = on_click(child, move |ctx, _| ctx.services.req::<Commands>().execute(command) );
    let node = enabled(node, C::enabled());
    let node = visibility(node, C::visible().map(Visibility::from));
    node
  }
}
// 
// How to use (Receive)
//
#[property(event)]
pub fn on_copy(child: impl UiNode, can_run: impl FnMut(&mut WidgetContext) -> bool, run: impl FnMut(&mut WidgetContext, CommandArgs)) -> impl UiNode {
  struct OnCopyNode<C, Q, R> {
    child: C,
    can_run: Q,
    run: R,
    handler: CommandHandler,
  }
  impl UiNode for OnCopyNode {
    fn init(&mut self, ctx: &mut WidgetContext) {
      self.handler = ctx.services.req::<Commands>().handler::<CopyCommand>();
      if self.can_run(ctx) {
        self.handler.set(true);
      }
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
      if self.can_run(ctx) {
        if self.handler.run_requested() {
          self.run(ctx);
        }
      } else {
        self.handler.set(false);
      }
    }
  }
}

#[widget($crate::text_box)]
pub mod text_box {
  properties! {
    on_copy = {
      can_run: |ctx| IsEnabled::get(ctx.vars) && IsVisible::get(ctx.vars),
      run: |ctx| {
        let text = ctx.widget_state[TextBoxTextState];
        ctx.services.req::<Clipboard>().set_text(text);
      }
    };
  }
}


// 
// How to bind shortcuts
//

fn main() {
  App::default().run(|ctx| {
    CopyCommand::shortcuts().set(ctx.vars, shortcut![Ctrl+C]);
  })
}