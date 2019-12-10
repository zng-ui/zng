use crate::core::*;

/// [View] presenter function result.
pub enum ViewUi<U: Ui> {
    /// Changes the view Ui to the new Ui associated.
    Update(U),
    /// Does not change the view Ui.
    Same,
}

/// Dynamically presents a data variable.
pub struct View<D: 'static, U: Ui, P: FnMut(&Var<D>, &mut NextUpdate) -> ViewUi<U>> {
    data: Var<D>,
    child: U,
    presenter: P,
}

#[impl_ui_crate(child)]
impl<D: 'static, U: Ui, P: FnMut(&Var<D>, &mut NextUpdate) -> ViewUi<U> + 'static> View<D, U, P> {
    pub fn new(data: Var<D>, initial_ui: U, presenter: P) -> Self {
        View {
            data,
            child: initial_ui,
            presenter,
        }
    }

    fn update_child(&mut self, update: &mut NextUpdate) {
        if let ViewUi::Update(new_child) = (self.presenter)(&self.data, update) {
            self.child = new_child;
            update.update_layout();
        }
    }

    #[Ui]
    fn init(&mut self, _values: &mut UiValues, update: &mut NextUpdate) {
        self.update_child(update);
    }

    #[Ui]
    fn value_changed(&mut self, _values: &mut UiValues, update: &mut NextUpdate) {
        if self.data.touched() {
            self.update_child(update);
        }
    }
}

/// Dynamically presents a data variable.
///
/// # Arguments
/// * `data`: Data variable that is presented by this view.
/// * `initial_ui`: Ui shown before the first presenter call.
/// * `presenter`: A function that generates Ui from `data`.
///
/// # Usage
/// `presenter` is called on init and every time `data` changes, the function returns a [ViewUi]
/// that is used by the view to present the data.
///
/// # Example
/// ```
/// # mod example {
/// # use crate::primitive::*;
/// fn countdown(n: Var<usize>) -> impl Ui {
///     enum State {
///         Blank,
///         Count,
///         End,
///     }
///
///     let mut state = State::Blank;
///
///     view(n, Ui3::A(text("starting..")), move |n, _| match state {
///         State::Blank => {
///             state = State::Count;
///             ViewUi::Update(Ui3::B(text(n.map(|n| format!("{}!", n).into()))))
///         }
///         State::Count => {
///             if **n > 0 {
///                 ViewUi::Same
///             } else {
///                 state = State::End;
///                 ViewUi::Update(Ui3::C(ui! {
///                     text_color: rgb(0, 0, 128);
///                     => text("Congratulations!!")
///                 }))
///             }
///         }
///         State::End => ViewUi::Same,
///     })
/// }
/// # }
/// ```
pub fn view<D: 'static, U: Ui, P: FnMut(&Var<D>, &mut NextUpdate) -> ViewUi<U> + 'static>(
    data: Var<D>,
    initial_ui: U,
    presenter: P,
) -> View<D, U, P> {
    View::new(data, initial_ui, presenter)
}
