//! Special `Var<Response>` type impls

use crate::{AnyVar, Var};

use super::*;

/// New paired [`ResponderVar`] and [`ResponseVar`] in the waiting state.
pub fn response_var<T: VarValue>() -> (ResponderVar<T>, ResponseVar<T>) {
    let responder = var(Response::Waiting::<T>);
    let response = responder.read_only();
    (ResponderVar(responder), ResponseVar(response))
}

/// New [`ResponseVar`] in the done state.
pub fn response_done_var<T: VarValue>(response: T) -> ResponseVar<T> {
    ResponseVar(var(Response::Done(response)).read_only())
}

/// Represents a read-write variable used to notify the completion of an async operation.
///
/// Use [`response_var`] to init.
#[derive(Clone)]
pub struct ResponderVar<T: VarValue>(Var<Response<T>>);

/// Represents a read-only variable used to listen to a one time signal that an async operation has completed.
///
/// Use [`response_var`] or [`response_done_var`] to init.
#[derive(Clone)]
pub struct ResponseVar<T: VarValue>(Var<Response<T>>);

impl<T: VarValue> ops::Deref for ResponderVar<T> {
    type Target = Var<Response<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: VarValue> IntoVar<Response<T>> for ResponderVar<T> {
    fn into_var(self) -> Var<Response<T>> {
        self.0
    }
}
impl<T: VarValue> From<ResponderVar<T>> for Var<Response<T>> {
    fn from(var: ResponderVar<T>) -> Self {
        var.0
    }
}
impl<T: VarValue> From<ResponderVar<T>> for AnyVar {
    fn from(var: ResponderVar<T>) -> Self {
        var.0.into()
    }
}

impl<T: VarValue> ops::Deref for ResponseVar<T> {
    type Target = Var<Response<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: VarValue> IntoVar<Response<T>> for ResponseVar<T> {
    fn into_var(self) -> Var<Response<T>> {
        self.0
    }
}
impl<T: VarValue> From<ResponseVar<T>> for Var<Response<T>> {
    fn from(var: ResponseVar<T>) -> Self {
        var.0
    }
}
impl<T: VarValue> From<ResponseVar<T>> for AnyVar {
    fn from(var: ResponseVar<T>) -> Self {
        var.0.into()
    }
}

/// Raw value in a [`ResponseVar`].
#[derive(Clone, Copy, PartialEq)]
pub enum Response<T: VarValue> {
    /// Responder has not set the response yet.
    Waiting,
    /// Responder has set the response.
    Done(T),
}
impl<T: VarValue> Response<T> {
    /// Has response.
    pub fn is_done(&self) -> bool {
        matches!(self, Response::Done(_))
    }

    /// Does not have response.
    pub fn is_waiting(&self) -> bool {
        matches!(self, Response::Waiting)
    }

    /// Gets the response if done.
    pub fn done(&self) -> Option<&T> {
        match self {
            Response::Waiting => None,
            Response::Done(r) => Some(r),
        }
    }
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
impl<T: VarValue> From<Response<T>> for Option<T> {
    fn from(value: Response<T>) -> Self {
        match value {
            Response::Waiting => None,
            Response::Done(r) => Some(r),
        }
    }
}
impl<T: VarValue> From<Response<Option<T>>> for Option<T> {
    fn from(value: Response<Option<T>>) -> Self {
        match value {
            Response::Waiting => None,
            Response::Done(r) => r,
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
    pub fn with_new_rsp<R>(&self, read: impl FnOnce(&T) -> R) -> Option<R> {
        self.with_new(|value| match value {
            Response::Waiting => None,
            Response::Done(value) => Some(read(value)),
        })
        .flatten()
    }

    /// If the response is received.
    pub fn is_done(&self) -> bool {
        self.with(Response::is_done)
    }

    /// If the response is not received yet.
    pub fn is_waiting(&self) -> bool {
        self.with(Response::is_waiting)
    }

    /// Clone the response value, if present.
    pub fn rsp(&self) -> Option<T> {
        self.with_rsp(Clone::clone)
    }

    /// Returns a future that awaits until a response is received and then returns a clone.
    pub async fn wait_rsp(&self) -> T {
        self.wait_done().await;
        self.rsp().unwrap()
    }

    /// Returns a future that awaits until a response is received.
    ///
    /// [`rsp`]: Self::rsp
    pub async fn wait_done(&self) {
        self.wait_match(Response::is_done).await;
    }

    /// Clone the response, if present and new.
    pub fn rsp_new(&self) -> Option<T> {
        self.with_new_rsp(Clone::clone)
    }

    /// Map the response value using `map`, if the variable is awaiting a response uses the `waiting_value` first.
    pub fn map_rsp<O, I, M>(&self, waiting_value: I, map: M) -> Var<O>
    where
        O: VarValue,
        I: Fn() -> O + Send + Sync + 'static,
        M: FnOnce(&T) -> O + Send + 'static,
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

    /// Map to another response variable.
    pub fn map_response<O, M>(&self, mut map: M) -> ResponseVar<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
    {
        ResponseVar(self.map(move |r| match r {
            Response::Waiting => Response::Waiting,
            Response::Done(t) => Response::Done(map(t)),
        }))
    }
}
impl<T: VarValue> IntoFuture for ResponseVar<T> {
    type Output = T;

    // refactor after 'impl_trait_in_assoc_type' is stable
    type IntoFuture = std::pin::Pin<Box<dyn Future<Output = T> + Send + Sync>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.wait_rsp().await })
    }
}

impl<T: VarValue> ResponderVar<T> {
    /// Sets the one time response.
    pub fn respond(&self, response: T) {
        self.set(Response::Done(response));
    }

    /// Creates a [`ResponseVar`] linked to this responder.
    pub fn response_var(&self) -> ResponseVar<T> {
        ResponseVar(self.read_only())
    }
}
