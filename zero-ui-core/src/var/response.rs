use super::*;

/// New paired [`ResponderVar`] and [`ResponseVar`] in the waiting state.
pub fn response_var<T: VarValue>() -> (ResponderVar<T>, ResponseVar<T>) {
    let responder = var(Response::Waiting::<T>);
    let response = responder.read_only();
    (responder, response)
}

/// New [`ResponseVar`] in the done state.
pub fn response_done_var<T: VarValue>(response: T) -> ResponseVar<T> {
    var(Response::Done(response)).read_only()
}

/// Variable used to notify the completion of an UI operation.
///
/// Use [`response_var`] to init.
pub type ResponderVar<T> = RcVar<Response<T>>;

/// Variable used to listen to a one time signal that an UI operation has completed.
///
/// Use [`response_var`] or [`response_done_var`] to init.
pub type ResponseVar<T> = types::ReadOnlyVar<Response<T>, RcVar<Response<T>>>;

/// Raw value in a [`ResponseVar`] or [`ResponseSender`].
#[derive(Clone, Copy)]
pub enum Response<T: VarValue> {
    /// Responder has not set the response yet.
    Waiting,
    /// Responder has set the response.
    Done(T),
}
impl<T: VarValue> fmt::Debug for Response<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            match self {
                Response::Waiting => {
                    write!(f, "Response::Waiting")
                }
                Response::Done(v) => f.debug_tuple("Response::Done").field(v).finish(),
            }
        } else {
            match self {
                Response::Waiting => {
                    write!(f, "Waiting")
                }
                Response::Done(v) => fmt::Debug::fmt(v, f),
            }
        }
    }
}

impl<T: VarValue> ResponseVar<T> {
    /// Visit the response, if present.
    pub fn with_rsp<R>(&self, read: impl FnOnce(&T) -> R) -> Option<R> {
        self.with(|value| match value {
            Response::Waiting => None,
            Response::Done(value) => Some(read(value)),
        })
    }

    /// Visit the response, if present and new.
    pub fn with_new_rsp<V: WithVars, R>(&self, vars: &V, read: impl FnOnce(&T) -> R) -> Option<R> {
        self.with_new(vars, |value| match value {
            Response::Waiting => None,
            Response::Done(value) => Some(read(value)),
        })
        .flatten()
    }

    /// Clone the response value, if present.
    pub fn rsp(&self) -> Option<T> {
        self.with_rsp(Clone::clone)
    }

    /// Returns a future that awaits until a response is received.
    ///
    /// Note that you must call [`rsp`] after this to get the response.
    ///
    /// [`rsp`]: Self::rsp
    pub async fn wait_rsp<Vr: WithVars>(&self, vars: &Vr) {
        while self.with_rsp(|_| false).unwrap_or(true) {
            self.wait_new(vars).await;
        }
    }

    /// Clone the response, if present and new.
    pub fn rsp_new<V: WithVars>(&self, vars: &V) -> Option<T> {
        self.with_new_rsp(vars, Clone::clone)
    }

    /// If the variable contains a response.
    pub fn responded(&self) -> bool {
        self.with(|value| match value {
            Response::Waiting => false,
            Response::Done(_) => true,
        })
    }

    /// Add a `handler` that is called once when the response is received,
    /// the handler is called before all other UI updates.
    ///
    /// [touched]: Var::touch
    /// [`responded`]: Self::responded
    pub fn on_pre_rsp<H>(&self, mut handler: H) -> VarHandle
    where
        H: AppHandler<T>,
    {
        if self.responded() {
            todo!()
        }

        self.on_pre_new(app_hn!(|ctx, args, handler_args| {
            if let Response::Done(value) = args {
                handler.event(
                    ctx,
                    value,
                    &crate::handler::AppHandlerArgs {
                        handle: handler_args,
                        is_preview: true,
                    },
                )
            }
        }))
    }

    /// Add a `handler` that is called once when the response is received,
    /// the handler is called after all other UI updates.
    ///
    /// [touched]: Var::touch
    /// [`responded`]: Self::responded
    pub fn on_rsp<H>(&self, mut handler: H) -> VarHandle
    where
        H: AppHandler<T>,
    {
        if self.responded() {
            todo!()
        }

        self.on_new(app_hn!(|ctx, args, handler_args| {
            if let Response::Done(value) = args {
                handler.event(
                    ctx,
                    value,
                    &crate::handler::AppHandlerArgs {
                        handle: handler_args,
                        is_preview: false,
                    },
                )
            }
        }))
    }

    /// Map the response value using `map`, if the variable is awaiting a response uses the `waiting_value` first.
    pub fn map_rsp<O, I, M>(&self, waiting_value: I, map: M) -> impl Var<O>
    where
        O: VarValue,
        I: Fn() -> O + 'static,
        M: FnOnce(&T) -> O + 'static,
    {
        let mut map = Some(map);
        self.filter_map(
            move |r| match r {
                Response::Waiting => None,
                Response::Done(r) => map.take().map(|m| m(r)),
            },
            waiting_value,
        )
    }
}

impl<T: VarValue> ResponderVar<T> {
    /// Sets the one time response.
    pub fn respond<'a, Vw: WithVars>(&'a self, vars: &'a Vw, response: T) {
        vars.with_vars(|vars| {
            self.set(vars, Response::Done(response));
        })
    }

    /// Creates a [`ResponseVar`] linked to this responder.
    pub fn response_var(&self) -> ResponseVar<T> {
        self.read_only()
    }
}
