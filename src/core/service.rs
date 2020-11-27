//! Services API.

use super::context::WindowContext;
use super::AnyMap;
use fnv::FnvHashSet;
use std::{any::*, cell::Cell, fmt, ptr, rc::Rc, thread::LocalKey};

/// Auto implement [`AppService`] trait.
pub use zero_ui_macros::AppService;

/// Identifies an application level service type.
///
/// # Derive
///
/// Implement this trait using `#[derive(AppService)]`.
pub trait AppService: 'static {
    /// Use `#[derive(AppService)]` to implement this trait.
    ///
    /// If that is not possible copy the `thread_local` implementation generated
    // by the macro as close as possible.
    #[doc(hidden)]
    fn thread_local_entry() -> AppServiceEntry<Self>
    where
        Self: Sized;
}

/// See [`AppService::thread_local_entry`].
#[doc(hidden)]
pub struct AppServiceValue<S: AppService> {
    value: Cell<*mut S>,
    assert_count: Rc<()>,
}
impl<S: AppService> AppServiceValue<S> {
    pub fn init() -> Self {
        AppServiceValue {
            value: Cell::new(ptr::null_mut()),
            assert_count: Rc::new(()),
        }
    }
}

/// See [`AppService::thread_local_entry`].
#[doc(hidden)]
pub struct AppServiceEntry<S: AppService> {
    local: &'static LocalKey<AppServiceValue<S>>,
}
impl<S: AppService> AppServiceEntry<S> {
    pub fn new(local: &'static LocalKey<AppServiceValue<S>>) -> Self {
        AppServiceEntry { local }
    }

    fn init(&self, service: *mut S) -> *mut S {
        self.local.with(move |l| l.value.replace(service))
    }

    fn get(&self) -> *mut S {
        self.local.with(|l| l.value.get())
    }

    fn assert_no_dup(&self) -> Rc<()> {
        let count = self.local.with(|l| Rc::clone(&l.assert_count));
        if Rc::strong_count(&count) == 1 {
            count
        } else {
            panic!("service `{}` already in query", type_name::<S>())
        }
    }
}

/// Identifies a window level service type.
pub trait WindowService: 'static {}

mod protected {
    use super::AppServices;
    use std::any::*;

    pub trait TypeBundle<'m> {
        type Borrowed;
        fn type_ids() -> Box<[TypeId]>;
        fn type_names() -> Box<[&'static str]>;
        fn downcast_mut(instances: Vec<&'m mut Box<dyn Any>>) -> Self::Borrowed;
    }

    pub trait AppServicesTuple<'s> {
        type Borrowed;

        fn assert_no_dup();

        fn get(services: &'s mut AppServices) -> Result<Self::Borrowed, &'static str>;
    }
}

#[doc(hidden)]
pub trait WindowServicesTuple<'m>: protected::TypeBundle<'m> {}

#[doc(hidden)]
pub trait AppServicesTuple<'s>: protected::AppServicesTuple<'s> {}

macro_rules! impl_AppServicesTuple {
    ( $( ( $($n:tt),+ ) ),+  $(,)?) => {$(paste::paste!{
        impl_AppServicesTuple! {
            impl $([<_borrowed $n>], [<ptr $n>] = [<S $n>]),+
        }
    })+};

    (impl $($assert:tt, $ptr:tt = $S:tt),+ ) => {
        impl<'s, $($S: AppService),+> protected::AppServicesTuple<'s> for ( $($S),+ ) {
            type Borrowed = ( $(&'s mut $S),+ );

            fn assert_no_dup() {
                $(
                    let $assert = $S::thread_local_entry().assert_no_dup();
                )+
            }

            fn get(_: &'s mut AppServices) -> Result<Self::Borrowed, &'static str> {
                Self::assert_no_dup();

                $(
                    let $ptr = $S::thread_local_entry().get();
                    if $ptr.is_null() {
                        return Err(type_name::<$S>());
                    }
                )+

                Ok(unsafe {($(
                    &mut *$ptr,
                )+)})
            }
        }

        impl<'s, $($S: AppService),+> AppServicesTuple<'s> for ( $($S),+ ) { }
    }
}
impl_AppServicesTuple! {
    (0, 1),
    (0, 1, 2),
    (0, 1, 2, 3),
    (0, 1, 2, 3, 4),
    (0, 1, 2, 3, 4, 5),
    (0, 1, 2, 3, 4, 5, 6),
    (0, 1, 2, 3, 4, 5, 6, 7),

    (0, 1, 2, 3, 4, 5, 6, 7, 8),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15),

    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23),

    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30),
    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31),
}

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
    // pub fn insert<S: 'static>(&mut self, service: S) {
    //     self.m.insert(TypeId::of::<S>(), Box::new(service));
    // }

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
            m: AppServices {
                services: Vec::with_capacity(20),
            },
        }
    }
}
impl AppServicesInit {
    /// Register a new service for the duration of the application context.
    pub fn register<S: AppService + Sized>(&mut self, service: S) -> Result<(), AlreadyRegistered> {
        let mut service = Box::new(service);
        let prev = S::thread_local_entry().init(service.as_mut() as _);
        if prev.is_null() {
            self.m.services.push(service);
            Ok(())
        } else {
            S::thread_local_entry().init(prev);
            Err(AlreadyRegistered {
                type_name: type_name::<S>(),
            })
        }
    }

    /// Moves the registered services into a new [`AppServices`].
    pub fn services(&mut self) -> &mut AppServices {
        &mut self.m
    }
}

/// Error when an app service or event of the same type is registered twice.
#[derive(Debug, Clone, Copy)]
pub struct AlreadyRegistered {
    /// Type name of the service.
    pub type_name: &'static str,
}
impl fmt::Display for AlreadyRegistered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is already registered", self.type_name)
    }
}
impl std::error::Error for AlreadyRegistered {}

/// Access to application services.
pub struct AppServices {
    services: Vec<Box<dyn AppService>>,
}
impl AppServices {
    /// Gets a service reference if the service is registered in the application.
    pub fn get<S: AppService>(&mut self) -> Option<&mut S> {
        let ptr = S::thread_local_entry().get();
        if ptr.is_null() {
            None
        } else {
            // SAFETY: This is safe as long as only AppServices calls thread_local_entry
            // with a &mut self reference.
            Some(unsafe { &mut *ptr })
        }
    }

    // Requires a service reference.
    ///
    /// # Panics
    ///
    /// If  the service is not registered in the application.
    pub fn req<S: AppService>(&mut self) -> &mut S {
        self.get::<S>()
            .unwrap_or_else(|| panic!("service `{}` is required", type_name::<S>()))
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
        M::get(self).ok()
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
        M::get(self).unwrap_or_else(|e| panic!("service `{}` is required", e))
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
    pub fn get_multi<'m, M: WindowServicesTuple<'m>>(&'m mut self) -> Option<M::Borrowed> {
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
    pub fn req_multi<'m, M: WindowServicesTuple<'m>>(&'m mut self) -> M::Borrowed {
        self.m.ret_multi::<M>()
    }
}
