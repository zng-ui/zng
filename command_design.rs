// 
// How to declare
//
mod declare_stuff{
  trait Command: 'static {
    fn enabled() -> Var<bool> {}
    fn visible() -> Var<bool> {}
  }

  struct Commands {}
  impl Commands { // Sender
    fn execute<C: Command>(&mut self, command: C) {
      // emits command event
    }
    fn enabled<C: Command>(&mut self, command: C) -> Var<bool> {

    }
    fn visible<C: Command>(&mut self, command: C) -> Var<bool> {

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




// 
// How to bind shortcuts
//


