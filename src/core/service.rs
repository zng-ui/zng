//! Services API.

use super::context::WindowContext;
use std::{
    any::*,
    cell::{Cell, RefCell},
    fmt, ptr,
    rc::Rc,
    thread::LocalKey,
};

/// Auto implement [`AppService`] trait.
use fnv::FnvHashSet;
pub use zero_ui_macros::{AppService, WindowService};

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

    /// Reference the [`AppServices`].
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
    pub fn get_multi<'m, M: AppServiceTuple<'m>>(&'m mut self) -> Option<M::Borrowed> {
        M::get().ok()
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
    pub fn req_multi<'m, M: AppServiceTuple<'m>>(&'m mut self) -> M::Borrowed {
        M::get().unwrap_or_else(|e| panic!("service `{}` is required", e))
    }
}

/// Window services registration.
#[derive(Default)]
pub struct WindowServicesInit {
    registered: FnvHashSet<TypeId>,
    #[allow(clippy::type_complexity)] // its a vec of boxed Fn(&WindowContext) -> (services, loaders, unloaders).
    builders: Vec<Box<dyn Fn(&WindowContext) -> (Box<dyn WindowService>, Box<dyn Fn()>, Box<dyn Fn()>)>>,
    #[allow(clippy::type_complexity)] // its a vec of boxed FnMut(&mut WindowContext), in a RefCell.
    visitors: RefCell<Vec<Box<dyn FnMut(&mut WindowContext)>>>,
}
impl WindowServicesInit {
    /// Register a new window service initializer.
    ///
    /// Window services have different instances for each window and exist for the duration
    /// of that window. The `new` closure is called for each new window.
    ///
    /// Services registered only apply in windows opened after.
    pub fn register<S: WindowService>(&mut self, new: impl Fn(&WindowContext) -> S + 'static) -> Result<(), AlreadyRegistered> {
        if !self.registered.insert(TypeId::of::<S>()) {
            return Err(AlreadyRegistered {
                type_name: type_name::<S>(),
            });
        }
        self.builders.push(Box::new(move |ctx| {
            let mut service = Box::new(new(ctx));
            let service_ptr = service.as_mut() as *mut S;
            let loader = Box::new(move || {
                let _ = S::thread_local_entry().init(service_ptr);
            });
            let unloader = Box::new(|| {
                let _ = S::thread_local_entry().init(ptr::null_mut());
            });
            (service, loader, unloader)
        }));

        Ok(())
    }

    /// Schedules a visitor that is called once for each open window.
    pub fn visit<V: FnMut(super::window::WindowId, &mut WindowServices) + 'static>(&self, mut visitor: V) {
        self.visitors.borrow_mut().push(Box::new(move |ctx| {
            visitor(ctx.window_id.get(), ctx.window_services);
        }));
    }

    /// Initializes services for a window context.
    ///
    /// # Using Services
    ///
    /// The window services are only available inside a call to [`AppContext::window_context`]. All
    /// the accessor methods panic if you attempt to request a service outside of the method.
    pub fn init(&self, ctx: &WindowContext) -> WindowServices {
        let mut services = Vec::with_capacity(self.builders.len());
        let mut loaders = Vec::with_capacity(self.builders.len());
        let mut unloaders = Vec::with_capacity(self.builders.len());

        for builder in &self.builders {
            let (service, loader, unloader) = builder(ctx);
            services.push(service);
            loaders.push(loader);
            unloaders.push(unloader);
        }

        WindowServices {
            _services: services,
            loaders,
            unloaders,
            loaded: false,
        }
    }

    pub(super) fn visitors(&mut self) -> &mut [Box<dyn FnMut(&mut WindowContext)>] {
        self.visitors.get_mut()
    }
}

/// Access to window services.
pub struct WindowServices {
    // hold the services alive.
    _services: Vec<Box<dyn WindowService>>,
    loaders: Vec<Box<dyn Fn()>>,
    unloaders: Vec<Box<dyn Fn()>>,
    loaded: bool,
}
impl WindowServices {
    pub(super) fn new() -> Self {
        Self {
            _services: vec![],
            loaders: vec![],
            unloaders: vec![],
            loaded: false,
        }
    }

    fn assert_loaded(&mut self) {
        assert!(self.loaded, "window services is not loaded in a WindowContext");
    }

    pub(super) fn load(&mut self) {
        if self.loaded {
            panic!("window services already loaded");
        }
        for load in &self.loaders {
            load();
        }
        self.loaded = true;
    }

    pub(super) fn unload(&mut self) {
        for unload in &self.unloaders {
            unload();
        }
        self.loaded = false;
    }

    /// Gets a service reference if the service is registered in the application.
    pub fn get<S: WindowService>(&mut self) -> Option<&mut S> {
        self.assert_loaded();

        let ptr = S::thread_local_entry().get();
        if ptr.is_null() {
            None
        } else {
            // SAFETY: This is safe as long as only WindowService calls thread_local_entry
            // with a &mut self reference.
            Some(unsafe { &mut *ptr })
        }
    }

    // Requires a service reference.
    ///
    /// # Panics
    ///
    /// If  the service is not registered in application.
    pub fn req<S: WindowService>(&mut self) -> &mut S {
        self.assert_loaded();

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
    pub fn get_multi<'m, M: WindowServiceTuple<'m>>(&'m mut self) -> Option<M::Borrowed> {
        self.assert_loaded();

        M::get().ok()
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
    pub fn req_multi<'m, M: WindowServiceTuple<'m>>(&'m mut self) -> M::Borrowed {
        self.assert_loaded();

        M::get().unwrap_or_else(|e| panic!("service `{}` is required", e))
    }
}

mod protected {
    pub trait AppServiceTuple<'s> {
        type Borrowed;
        fn assert_no_dup();
        fn get() -> Result<Self::Borrowed, &'static str>;
    }
    pub trait WindowServiceTuple<'s> {
        type Borrowed;
        fn assert_no_dup();
        fn get() -> Result<Self::Borrowed, &'static str>;
    }
}
macro_rules! impl_multi_tuple {
    ($Trait:ident, $Tuple:ident => $( ( $($n:tt),+ ) ),+  $(,)?) => {$(paste::paste!{
        impl_multi_tuple! {
            impl $Trait, $Tuple $([<_borrowed $n>], [<ptr $n>] = [<S $n>]),+
        }
    })+};

    (impl $Trait:ident, $Tuple:ident $($assert:tt, $ptr:tt = $S:tt),+ ) => {
        impl<'s, $($S: $Trait),+> protected::$Tuple<'s> for ( $($S),+ ) {
            type Borrowed = ( $(&'s mut $S),+ );

            fn assert_no_dup() {
                $(
                    let $assert = $S::thread_local_entry().assert_no_dup();
                )+
            }

            fn get() -> Result<Self::Borrowed, &'static str> {
                Self::assert_no_dup();

                $(
                    let $ptr = $S::thread_local_entry().get();
                    if $ptr.is_null() {
                        return Err(type_name::<$S>());
                    }
                )+

                // SAFETY: assert_no_dup validated that all pointers are unique.
                // The cast to &mut is safe as long as it's only called in AppServices::get_multi().
                Ok(unsafe {($(
                    &mut *$ptr,
                )+)})
            }
        }

        impl<'s, $($S: $Trait),+> $Tuple<'s> for ( $($S),+ ) { }
    }
}
macro_rules! service_types {
    ($(
        $(#[$doc:meta])*
        pub trait $Trait:ident { }
    )+) => {$(paste::paste! {
        $(#[$doc])*
        ///
        /// # Derive
        ///
        #[doc="Implement this trait using `#[derive(" $Trait ")].`"]
        pub trait $Trait: 'static {
            /// Use `#[derive ..]` to implement this trait.
            ///
            /// If that is not possible copy the `thread_local` implementation generated
            // by the macro as close as possible.
            #[doc(hidden)]
            fn thread_local_entry() -> [<$Trait Entry>]<Self>
            where
                Self: Sized;
        }

        #[doc(hidden)]
        pub struct [<$Trait Value>]<S: $Trait> {
            value: Cell<*mut S>,
            assert_count: Rc<()>
        }
        impl<S: $Trait> [<$Trait Value>]<S> {
            pub fn init() -> Self {
                Self { value: Cell::new(ptr::null_mut()), assert_count: Rc::new(()) }
            }
        }

        #[doc(hidden)]
        pub struct [<$Trait Entry>]<S: $Trait> {
            local: &'static LocalKey<[<$Trait Value>]<S>>,
        }

        impl<S: $Trait> [<$Trait Entry>]<S> {
            pub fn new(local: &'static LocalKey<[<$Trait Value>]<S>>) -> Self {
                Self { local }
            }

            fn init(&self, service: *mut S) -> *mut S {
                self.local.with(move |l| l.value.replace(service))
            }

            fn get(&self) -> *mut S {
                self.local.with(|l| l.value.get())
            }

            fn assert_no_dup(&self) -> Rc<()> {
                let count = self.local.with(|l| Rc::clone(&l.assert_count));
                if Rc::strong_count(&count) == 2 {
                    count
                } else {
                    panic!("service `{}` already in query", type_name::<S>())
                }
            }
        }

        #[doc(hidden)]
        pub trait [<$Trait Tuple>]<'s>: protected::[<$Trait Tuple>]<'s> { }

        impl_multi_tuple! {
            $Trait, [<$Trait Tuple>] =>
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
    })+};
}

service_types! {
    /// Identifies an application level service type.
    pub trait AppService { }

    /// Identifies a window level service type.
    pub trait WindowService { }
}
