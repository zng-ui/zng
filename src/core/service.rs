//! Services API.

use super::context::WindowContext;
use super::AnyMap;
use fnv::FnvHashSet;
use std::any::*;

/// Identifies an application level service type.
pub trait AppService: 'static {}

/// Identifies a window level service type.
pub trait WindowService: 'static {}

mod protected {
    use std::any::*;
    pub trait TypeBundle<'m> {
        type Borrowed;
        fn type_ids() -> Box<[TypeId]>;
        fn type_names() -> Box<[&'static str]>;
        fn downcast_mut(instances: Vec<&'m mut Box<dyn Any>>) -> Self::Borrowed;
    }
}
#[doc(hidden)]
pub trait AppServicesTuple<'m>: protected::TypeBundle<'m> {}
#[doc(hidden)]
pub trait WindowServicesTuple<'m>: protected::TypeBundle<'m> {}

macro_rules! impl_type_bundle {
    ($N:expr, $T:ident) => {
        //DONE
    };
    ($N:expr, $TLast:ident, $($T:ident),+) => {
        impl_type_bundle!($N - 1, $($T),+);
        impl_type_bundle!(impl $N, next -> $TLast, $(next -> $T),+);
    };
    (impl $N: expr, $($next:ident -> $T:ident),+) => {
        impl<'m, $($T: 'static),+> protected::TypeBundle<'m> for ($($T),+) {
            type Borrowed = ($(&'m mut $T),+);

            fn type_ids() -> Box<[TypeId]> {
                Box::new([
                    $(TypeId::of::<$T>(),)+
                ])
            }

            fn type_names() -> Box<[&'static str]> {
                Box::new([
                    $(type_name::<$T>(),)+
                ])
            }

            fn downcast_mut(instances: Vec<&'m mut Box<dyn Any>>) -> Self::Borrowed {
                let mut instances = instances.into_iter();
                #[allow(non_snake_case)]
                match ($(instances.$next()),+) {
                    ($(Some($T)),+) => ($($T.downcast_mut::<$T>().unwrap()),+),
                    _ => panic!("expected {} instances", $N)
                }
            }
        }

        impl<'m, $($T: AppService),+> AppServicesTuple<'m> for ($($T),+) {}

        impl<'m, $($T: WindowService),+> WindowServicesTuple<'m> for ($($T),+) {}
    };
}

mod type_bundle_impls {
    use super::*;
    impl_type_bundle!(16, T15, T14, T13, T12, T11, T10, T9, T8, T7, T6, T5, T4, T3, T2, T1, T0);
}

#[derive(Default)]
struct ServiceMap {
    m: AnyMap,
}
impl ServiceMap {
    pub fn insert<S: 'static>(&mut self, service: S) {
        self.m.insert(TypeId::of::<S>(), Box::new(service));
    }

    pub fn get<S: 'static>(&mut self) -> Option<&mut S> {
        let type_id = TypeId::of::<S>();
        self.m.get_mut(&type_id).map(|any| any.downcast_mut::<S>().unwrap())
    }

    pub fn req<S: 'static>(&mut self) -> &mut S {
        self.get::<S>()
            .unwrap_or_else(|| panic!("service `{}` is required", type_name::<S>()))
    }

    fn borrow_multi<'m, M: protected::TypeBundle<'m>>(&'m mut self) -> Result<M::Borrowed, &'static str> {
        let mut unique = FnvHashSet::default();
        let type_ids = M::type_ids();
        let mut instances = Vec::with_capacity(type_ids.len());

        for (i, tid) in type_ids.iter().enumerate() {
            if unique.insert(tid) {
                if let Some(any) = self.m.get_mut(tid) {
                    let p = any as *mut _;
                    instances.push(unsafe { &mut *p });
                } else {
                    return Err(M::type_names()[i]);
                }
            } else {
                panic!("service `{}` already borrowed", M::type_names()[i]);
            }
        }

        Ok(M::downcast_mut(instances))
    }

    pub fn get_multi<'m, M: protected::TypeBundle<'m>>(&'m mut self) -> Option<M::Borrowed> {
        self.borrow_multi::<M>().ok()
    }

    pub fn ret_multi<'m, M: protected::TypeBundle<'m>>(&'m mut self) -> M::Borrowed {
        self.borrow_multi::<M>().unwrap_or_else(|s| panic!("service `{}` is required", s))
    }
}

/// Application services with registration access.
pub struct AppServicesInit {
    m: AppServices,
}
impl Default for AppServicesInit {
    fn default() -> Self {
        AppServicesInit {
            m: AppServices { m: ServiceMap::default() },
        }
    }
}
impl AppServicesInit {
    /// Register a new service for the duration of the application context.
    pub fn register<S: AppService>(&mut self, service: S) {
        self.m.m.insert(service)
    }

    /// Moves the registered services into a new [`AppServices`].
    pub fn services(&mut self) -> &mut AppServices {
        &mut self.m
    }
}

/// Access to application services.
pub struct AppServices {
    m: ServiceMap,
}
impl AppServices {
    /// Gets a service reference if the service is registered in the application.
    pub fn get<S: AppService>(&mut self) -> Option<&mut S> {
        self.m.get::<S>()
    }

    // Requires a service reference.
    ///
    /// # Panics
    /// If  the service is not registered in the application.
    pub fn req<S: AppService>(&mut self) -> &mut S {
        self.m.req::<S>()
    }

    /// Gets multiple service references if all services are registered in the application.
    ///
    /// # Service Types
    ///
    /// The type argument must be a tuple (2..=16) of [`AppService`] implementers. No type must repeat.
    /// The return type is a tuple with each service type borrowed mutable (`&mut S`).
    ///
    /// # Panics
    /// If the same service type is requested more then once.
    pub fn get_multi<'m, M: AppServicesTuple<'m>>(&'m mut self) -> Option<M::Borrowed> {
        self.m.get_multi::<M>()
    }

    /// Requires multiple service references.
    ///
    /// # Service Types
    ///
    /// The type argument must be a tuple (2..=16) of [`AppService`] implementers. No type must repeat.
    /// The return type is a tuple with each service type borrowed mutable (`&mut S`).
    ///
    /// # Panics
    /// If any of the services is not registered in the application.
    ///
    /// If the same service type is required more then once.
    pub fn req_multi<'m, M: AppServicesTuple<'m>>(&'m mut self) -> M::Borrowed {
        self.m.ret_multi::<M>()
    }
}

type WindowServicesBuilder = Vec<(TypeId, Box<dyn Fn(&WindowContext) -> Box<dyn Any>>)>;

/// Window services registration.
#[derive(Default)]
pub struct WindowServicesInit {
    builders: WindowServicesBuilder,
}
impl WindowServicesInit {
    /// Register a new window service initializer.
    ///
    /// Window services have different instances for each window and exist for the duration
    /// of that window. The `new` closure is called for each new window.
    ///
    /// Services registered only apply in windows opened after.
    pub fn register<S: WindowService>(&mut self, new: impl Fn(&WindowContext) -> S + 'static) {
        self.builders.push((TypeId::of::<S>(), Box::new(move |ctx| Box::new(new(ctx)))));
    }

    /// Initializes services for a window context.
    pub fn init(&self, ctx: &WindowContext) -> WindowServices {
        WindowServices {
            m: ServiceMap {
                m: self.builders.iter().map(|(k, v)| (*k, (v)(ctx))).collect(),
            },
        }
    }
}

/// Access to window services.
pub struct WindowServices {
    m: ServiceMap,
}
impl WindowServices {
    pub(super) fn new() -> Self {
        WindowServices { m: ServiceMap::default() }
    }

    /// Gets a service reference if the service is registered in the application.
    pub fn get<S: WindowService>(&mut self) -> Option<&mut S> {
        self.m.get::<S>()
    }

    // Requires a service reference.
    ///
    /// # Panics
    /// If  the service is not registered in application.
    pub fn req<S: WindowService>(&mut self) -> &mut S {
        self.m.req::<S>()
    }

    /// Gets multiple service references if all services are registered in the application.
    ///
    /// # Service Types
    ///
    /// The type argument must be a tuple (2..=16) of [`AppService`] implementers. No type must repeat.
    /// The return type is a tuple with each service type borrowed mutable (`&mut S`).
    ///
    /// # Panics
    ///
    /// If the same service type is requested more then once.
    pub fn get_multi<'m, M: AppServicesTuple<'m>>(&'m mut self) -> Option<M::Borrowed> {
        self.m.get_multi::<M>()
    }

    /// Requires multiple service references.
    ///
    /// # Service Types
    ///
    /// The type argument must be a tuple (2..=16) of [`AppService`] implementers. No type must repeat.
    /// The return type is a tuple with each service type borrowed mutable (`&mut S`).
    ///
    /// # Panics
    ///
    /// If any of the services is not registered in the application.
    ///
    /// If the same service type is required more then once.
    pub fn req_multi<'m, M: AppServicesTuple<'m>>(&'m mut self) -> M::Borrowed {
        self.m.ret_multi::<M>()
    }
}
