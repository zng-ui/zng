//!: Vars service.

use std::{mem, sync::Arc, thread::ThreadId, time::Duration};

use parking_lot::Mutex;
use smallbox::SmallBox;
use zng_app_context::{app_local, context_local};
use zng_time::{DInstant, Deadline, INSTANT, INSTANT_APP};
use zng_unit::{Factor, FactorUnits as _, TimeUnits as _};

use smallbox::smallbox;

use crate::{
    AnyVar, Var,
    animation::{Animation, AnimationController, AnimationHandle, AnimationTimer, ModifyInfo},
    var,
};

/// Represents the last time a variable was mutated or the current update cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, bytemuck::NoUninit)]
#[repr(transparent)]
pub struct VarUpdateId(pub(crate) u32);
impl VarUpdateId {
    /// ID that is never new.
    pub const fn never() -> Self {
        VarUpdateId(0)
    }

    fn next(&mut self) {
        if self.0 == u32::MAX {
            self.0 = 1;
        } else {
            self.0 += 1;
        }
    }
}
impl Default for VarUpdateId {
    fn default() -> Self {
        Self::never()
    }
}

pub(super) type VarUpdateFn = SmallBox<dyn FnMut() + Send + 'static, smallbox::space::S8>;

app_local! {
    pub(crate) static VARS_SV: VarsService = VarsService::new();
}
context_local! {
    pub(crate) static VARS_MODIFY_CTX: Option<ModifyInfo> = None;
    pub(crate) static VARS_ANIMATION_CTRL_CTX: Box<dyn AnimationController> = {
        let r: Box<dyn AnimationController> = Box::new(());
        r
    };
}

type AnimationFn = SmallBox<dyn FnMut(AnimationUpdateInfo) -> Option<Deadline> + Send, smallbox::space::S8>;

pub(crate) struct VarsService {
    // animation config
    animations_enabled: Var<bool>,
    sys_animations_enabled: Var<bool>,
    frame_duration: Var<Duration>,
    animation_time_scale: Var<Factor>,

    // VARS_APP stuff
    app_waker: Option<SmallBox<dyn Fn() + Send + Sync + 'static, smallbox::space::S2>>,
    modify_trace: Option<SmallBox<dyn Fn(&'static str) + Send + Sync + 'static, smallbox::space::S2>>,

    // AnyVar::perm storage
    perm: Mutex<Vec<AnyVar>>,

    // update state
    update_id: VarUpdateId,
    updates: Mutex<Vec<(ModifyInfo, VarUpdateFn)>>,
    updating_thread: Option<ThreadId>,
    updates_after: Mutex<Vec<(ModifyInfo, VarUpdateFn)>>,

    // animations state
    ans_animations: Mutex<Vec<AnimationFn>>,
    ans_animation_imp: usize,
    ans_current_modify: ModifyInfo,
    ans_animation_start_time: Option<DInstant>,
    ans_next_frame: Option<Deadline>,
}
impl VarsService {
    pub(crate) fn new() -> Self {
        let sys_animations_enabled = var(true);
        Self {
            animations_enabled: sys_animations_enabled.cow(),
            sys_animations_enabled,
            frame_duration: var((1.0 / 60.0).secs()),
            animation_time_scale: var(1.fct()),

            app_waker: None,
            modify_trace: None,

            perm: Mutex::new(vec![]),

            update_id: VarUpdateId::never(),
            updates: Mutex::new(vec![]),
            updating_thread: None,
            updates_after: Mutex::new(vec![]),

            ans_animations: Mutex::new(vec![]),
            ans_animation_imp: 0,
            ans_current_modify: ModifyInfo::never(),
            ans_animation_start_time: None,
            ans_next_frame: None,
        }
    }

    fn wake_app(&self) {
        if let Some(w) = &self.app_waker {
            w();
        }
    }
}

/// Variable updates and animation service.
pub struct VARS;
impl VARS {
    /// Id of the current vars update in the app scope.
    ///
    /// Variable with [`AnyVar::last_update`] equal to this are *new*.
    pub fn update_id(&self) -> VarUpdateId {
        VARS_SV.read().update_id
    }

    /// Read-write that defines if animations are enabled on the app.
    ///
    /// The value is the same as [`sys_animations_enabled`], if set the variable disconnects from system config.
    ///
    /// [`sys_animations_enabled`]: Self::sys_animations_enabled
    pub fn animations_enabled(&self) -> Var<bool> {
        VARS_SV.read().animations_enabled.clone()
    }

    /// Read-only that tracks if animations are enabled in the operating system.
    ///
    /// This is `true` by default, it updates when the operating system config changes.
    pub fn sys_animations_enabled(&self) -> Var<bool> {
        VARS_SV.read().sys_animations_enabled.read_only()
    }

    /// Variable that defines the global frame duration, the default is 60fps `(1.0 / 60.0).secs()`.
    pub fn frame_duration(&self) -> Var<Duration> {
        VARS_SV.read().frame_duration.clone()
    }

    /// Variable that defines a global scale for the elapsed time of animations.
    pub fn animation_time_scale(&self) -> Var<Factor> {
        VARS_SV.read().animation_time_scale.clone()
    }

    /// Info about the current context when requesting variable modification.
    ///
    /// If is currently inside a [`VARS.animate`] closure, or inside a [`Var::modify`] closure requested by an animation, or inside
    /// an [`AnimationController`], returns the info that was collected at the moment the animation was requested. Outside of animations
    /// gets an info with [`importance`] guaranteed to override the [`modify_importance`].
    ///
    /// [`importance`]: ModifyInfo::importance
    /// [`modify_importance`]: AnyVar::modify_importance
    /// [`AnimationController`]: crate::animation::AnimationController
    /// [`VARS.animate`]: VARS::animate
    pub fn current_modify(&self) -> ModifyInfo {
        match VARS_MODIFY_CTX.get_clone() {
            Some(current) => current, // override set by modify and animation closures.
            None => VARS_SV.read().ans_current_modify.clone(),
        }
    }

    /// Adds an `animation` closure that is called every frame to update captured variables, starting after next frame.
    ///
    /// This is used to implement all [`Var<T>`] animations, it enables any kind of variable animation,
    /// including multiple variables.
    ///
    /// Returns an [`AnimationHandle`] that can be used to monitor the animation status and to [`stop`] or to
    /// make the animation [`perm`].
    ///
    /// # Variable Control
    ///
    /// Animations assume *control* of a variable on the first time they cause its value to be new, after this
    /// moment the [`AnyVar::is_animating`] value is `true` and [`AnyVar::modify_importance`] is the animation's importance,
    /// until the animation stops. Only one animation can control a variable at a time, if an animation loses control of a
    /// variable all attempts to modify it from inside the animation are ignored.
    ///
    /// Later started animations steal control from previous animations, update, modify or set calls also remove the variable
    /// from being affected by a running animation, even if just set to an equal value, that is, not actually updated.
    ///
    /// # Nested Animations
    ///
    /// Other animations can be started from inside the animation closure, these *nested* animations have the same importance
    /// as the *parent* animation, the animation handle is different and [`AnyVar::is_animating`] is `false` if the nested animation
    /// is dropped before the *parent* animation. But because the animations share the same importance the parent animation can
    /// set the variable again.
    ///
    /// # Examples
    ///
    /// The example animates a `text` variable from `"Animation at 0%"` to `"Animation at 100%"`, when the animation
    /// stops the `completed` variable is set to `true`.
    ///
    /// ```
    /// # use zng_var::{*, animation::easing};
    /// # use zng_txt::*;
    /// # use zng_unit::*;
    /// # use zng_clone_move::*;
    /// #
    /// fn animate_text(text: &Var<Txt>, completed: &Var<bool>) {
    ///     let transition = animation::Transition::new(0u8, 100);
    ///     let mut prev_value = 101;
    ///     VARS.animate(clmv!(text, completed, |animation| {
    ///         let step = easing::expo(animation.elapsed_stop(1.secs()));
    ///         let value = transition.sample(step);
    ///         if value != prev_value {
    ///             if value == 100 {
    ///                 animation.stop();
    ///                 let _ = completed.set(true);
    ///             }
    ///             let _ = text.set(formatx!("Animation at {value}%"));
    ///             prev_value = value;
    ///         }
    ///     }))
    ///     .perm()
    /// }
    /// ```
    ///
    /// Note that the animation can be stopped from the inside, the closure parameter is an [`Animation`]. In
    /// the example this is the only way to stop the animation, because [`perm`] was called. Animations hold a clone
    /// of the variables they affect and exist for the duration of the app if not stopped, causing the app to wake and call the
    /// animation closure for every frame.
    ///
    /// This method is the most basic animation interface, used to build all other animations, its rare that you
    /// will need to use it directly, most of the time animation effects can be composted using the [`Var`] easing and mapping
    /// methods.
    ///
    /// ```
    /// # use zng_var::{*, animation::easing};
    /// # use zng_txt::*;
    /// # use zng_unit::*;
    /// # fn demo() {
    /// let value = var(0u8);
    /// let text = value.map(|v| formatx!("Animation at {v}%"));
    /// value.ease(100, 1.secs(), easing::expo);
    /// # }
    /// ```
    ///
    /// # Optimization Tips
    ///
    /// When no animation is running the app *sleeps* awaiting for an external event, update request or timer elapse, when at least one
    /// animation is running the app awakes every [`VARS.frame_duration`]. You can use [`Animation::sleep`] to *pause* the animation
    /// for a duration, if all animations are sleeping the app is also sleeping.
    ///
    /// Animations lose control over a variable permanently when a newer animation modifies the var or
    /// the var is modified directly, but even if the animation can't control any variables **it keeps running**.
    /// This happens because the system has no insight of all side effects caused by the `animation` closure. You
    /// can use the [`VARS.current_modify`] and [`AnyVar::modify_importance`] to detect when the animation no longer affects
    /// any variables and stop the animation to avoid awaking the app for no reason.
    ///
    /// These optimizations are already implemented by the animations provided as methods of [`Var<T>`].
    ///
    /// # External Controller
    ///
    /// The animation can be controlled from the inside using the [`Animation`] reference, it can be stopped using the returned
    /// [`AnimationHandle`], and it can also be controlled by a registered [`AnimationController`] that can manage multiple
    /// animations at the same time, see [`with_animation_controller`] for more details.
    ///
    /// [`Animation::sleep`]: Animation::sleep
    /// [`stop`]: AnimationHandle::stop
    /// [`perm`]: AnimationHandle::perm
    /// [`with_animation_controller`]: VARS::with_animation_controller
    /// [`VARS.frame_duration`]: VARS::frame_duration
    /// [`VARS.current_modify`]: VARS::current_modify
    pub fn animate<A>(&self, animation: A) -> AnimationHandle
    where
        A: FnMut(&Animation) + Send + 'static,
    {
        VARS.animate_impl(smallbox!(animation))
    }

    /// Calls `animate` while `controller` is registered as the animation controller.
    ///
    /// The `controller` is notified of animation events for each animation spawned by `animate` and can affect then with the same
    /// level of access as [`VARS.animate`]. Only one controller can affect animations at a time.
    ///
    /// This can be used to manage multiple animations at the same time, or to get [`VARS.animate`] level of access to an animation
    /// that is not implemented to allow such access. Note that animation implementers are not required to support the full
    /// [`Animation`] API, for example, there is no guarantee that a restart requested by the controller will repeat the same animation.
    ///
    /// The controller can start new animations, these animations will have the same controller if not overridden, you can
    /// use this method and the `()` controller to avoid this behavior.
    ///
    /// [`Animation`]: animation::Animation
    /// [`VARS.animate`]: VARS::animate
    pub fn with_animation_controller<R>(&self, controller: impl AnimationController, animate: impl FnOnce() -> R) -> R {
        let controller: Box<dyn AnimationController> = Box::new(controller);
        let mut opt = Some(Arc::new(controller));
        VARS_ANIMATION_CTRL_CTX.with_context(&mut opt, animate)
    }

    pub(crate) fn schedule_update(&self, value_type_name: &'static str, apply_update: impl FnOnce() + Send + 'static) {
        let mut once = Some(apply_update);
        self.schedule_update_impl(
            value_type_name,
            smallbox!(move || {
                let once = once.take().unwrap();
                once();
            }),
        );
    }

    pub(crate) fn perm(&self, var: AnyVar) {
        VARS_SV.read().perm.lock().push(var);
    }
}

/// VARS APP integration.
#[expect(non_camel_case_types)]
pub struct VARS_APP;
impl VARS_APP {
    /// Register a closure called when [`apply_updates`] should be called because there are changes pending.
    ///
    /// # Panics
    ///
    /// Panics if already called for the current app. This must be called by app framework implementers only.
    ///
    /// [`apply_updates`]: Self::apply_updates
    pub fn init_app_waker(&self, waker: impl Fn() + Send + Sync + 'static) {
        let mut vars = VARS_SV.write();
        assert!(vars.app_waker.is_none());
        vars.app_waker = Some(smallbox!(waker));
    }

    /// Register a closure called when a variable modify is about to be scheduled. The
    /// closure parameter is the type name of the variable type.
    ///
    /// # Panics
    ///
    /// Panics if already called for the current app. This must be called by app framework implementers only.
    pub fn init_modify_trace(&self, trace: impl Fn(&'static str) + Send + Sync + 'static) {
        let mut vars = VARS_SV.write();
        assert!(vars.modify_trace.is_none());
        vars.modify_trace = Some(smallbox!(trace));
    }

    /// If [`apply_updates`] will do anything.
    ///
    /// [`apply_updates`]: Self::apply_updates
    pub fn has_pending_updates(&self) -> bool {
        !VARS_SV.write().updates.get_mut().is_empty()
    }

    /// Sets the `sys_animations_enabled` read-only variable.
    pub fn set_sys_animations_enabled(&self, enabled: bool) {
        VARS_SV.read().sys_animations_enabled.set(enabled);
    }

    /// Apply all pending updates, call hooks and update bindings.
    ///
    /// This must be called by app framework implementers only.
    pub fn apply_updates(&self) {
        let _s = tracing::trace_span!("VARS").entered();
        let _t = INSTANT_APP.pause_for_update();
        VARS.apply_updates_and_after(0)
    }

    /// Does one animation frame if the frame duration has elapsed.
    ///
    /// This must be called by app framework implementers only.
    pub fn update_animations(&self, timer: &mut impl AnimationTimer) {
        VARS.update_animations_impl(timer);
    }

    /// Register the next animation frame, if there are any active animations.
    ///
    /// This must be called by app framework implementers only.
    pub fn next_deadline(&self, timer: &mut impl AnimationTimer) {
        VARS.next_deadline_impl(timer)
    }
}

impl VARS {
    fn schedule_update_impl(&self, value_type_name: &'static str, update: VarUpdateFn) {
        let vars = VARS_SV.read();
        if let Some(trace) = &vars.modify_trace {
            trace(value_type_name);
        }
        let cur_modify = match VARS_MODIFY_CTX.get_clone() {
            Some(current) => current, // override set by modify and animation closures.
            None => vars.ans_current_modify.clone(),
        };

        if let Some(id) = vars.updating_thread {
            if std::thread::current().id() == id {
                // is binding request, enqueue for immediate exec.
                vars.updates.lock().push((cur_modify, update));
            } else {
                // is request from app task thread when we are already updating, enqueue for exec after current update.
                vars.updates_after.lock().push((cur_modify, update));
            }
        } else {
            // request from any app thread,
            vars.updates.lock().push((cur_modify, update));
            vars.wake_app();
        }
    }

    fn apply_updates_and_after(&self, depth: u8) {
        let mut vars = VARS_SV.write();

        match depth {
            0 => {
                vars.update_id.next();
                vars.ans_animation_start_time = None;
            }
            10 => {
                // high-pressure from worker threads, skip
                return;
            }
            _ => {}
        }

        // updates requested by other threads while was applying updates
        let mut updates = mem::take(vars.updates_after.get_mut());
        // normal updates
        if updates.is_empty() {
            updates = mem::take(vars.updates.get_mut());
        } else {
            updates.append(vars.updates.get_mut());
        }
        // apply pending updates
        if !updates.is_empty() {
            debug_assert!(vars.updating_thread.is_none());
            vars.updating_thread = Some(std::thread::current().id());

            drop(vars);
            update_each_and_bindings(updates, 0);

            vars = VARS_SV.write();
            vars.updating_thread = None;

            if !vars.updates_after.get_mut().is_empty() {
                drop(vars);
                VARS.apply_updates_and_after(depth + 1)
            }
        }

        fn update_each_and_bindings(updates: Vec<(ModifyInfo, VarUpdateFn)>, depth: u16) {
            if depth == 1000 {
                tracing::error!(
                    "updated variable bindings 1000 times, probably stuck in an infinite loop\n\
                    will skip next updates"
                );
                return;
            }

            for (info, mut update) in updates {
                #[allow(clippy::redundant_closure)] // false positive
                VARS_MODIFY_CTX.with_context(&mut Some(Arc::new(Some(info))), || (update)());

                let mut vars = VARS_SV.write();
                let updates = mem::take(vars.updates.get_mut());
                if !updates.is_empty() {
                    drop(vars);
                    update_each_and_bindings(updates, depth + 1);
                }
            }
        }
    }

    fn animate_impl(&self, mut animation: SmallBox<dyn FnMut(&Animation) + Send + 'static, smallbox::space::S4>) -> AnimationHandle {
        let mut vars = VARS_SV.write();

        // # Modify Importance
        //
        // Variables only accept modifications from an importance (IMP) >= the previous IM that modified it.
        //
        // Direct modifications always overwrite previous animations, so we advance the IMP for each call to
        // this method **and then** advance the IMP again for all subsequent direct modifications.
        //
        // Example sequence of events:
        //
        // |IM| Modification  | Accepted
        // |--|---------------|----------
        // | 1| Var::set      | YES
        // | 2| Var::ease     | YES
        // | 2| ease update   | YES
        // | 3| Var::set      | YES
        // | 3| Var::set      | YES
        // | 2| ease update   | NO
        // | 4| Var::ease     | YES
        // | 2| ease update   | NO
        // | 4| ease update   | YES
        // | 5| Var::set      | YES
        // | 2| ease update   | NO
        // | 4| ease update   | NO

        // ensure that all animations started in this update have the same exact time, we update then with the same `now`
        // timestamp also, this ensures that synchronized animations match perfectly.
        let start_time = if let Some(t) = vars.ans_animation_start_time {
            t
        } else {
            let t = INSTANT.now();
            vars.ans_animation_start_time = Some(t);
            t
        };

        let mut anim_imp = None;
        if let Some(c) = VARS_MODIFY_CTX.get_clone() {
            if c.is_animating() {
                // nested animation uses parent importance.
                anim_imp = Some(c.importance);
            }
        }
        let anim_imp = match anim_imp {
            Some(i) => i,
            None => {
                // not nested, advance base imp
                let mut imp = vars.ans_animation_imp.wrapping_add(1);
                if imp == 0 {
                    imp = 1;
                }

                let mut next_imp = imp.wrapping_add(1);
                if next_imp == 0 {
                    next_imp = 1;
                }

                vars.ans_animation_imp = next_imp;
                vars.ans_current_modify.importance = next_imp;

                imp
            }
        };

        let (handle_owner, handle) = AnimationHandle::new();
        let weak_handle = handle.downgrade();

        let controller = VARS_ANIMATION_CTRL_CTX.get();

        let anim = Animation::new(vars.animations_enabled.get(), start_time, vars.animation_time_scale.get());

        drop(vars);

        controller.on_start(&anim);
        let mut controller = Some(controller);
        let mut anim_modify_info = Some(Arc::new(Some(ModifyInfo {
            handle: Some(weak_handle.clone()),
            importance: anim_imp,
        })));

        let mut vars = VARS_SV.write();

        vars.ans_animations.get_mut().push(smallbox!(move |info: AnimationUpdateInfo| {
            let _handle_owner = &handle_owner; // capture and own the handle owner.

            if weak_handle.upgrade().is_some() {
                if anim.stop_requested() {
                    // drop
                    controller.as_ref().unwrap().on_stop(&anim);
                    return None;
                }

                if let Some(sleep) = anim.sleep_deadline() {
                    if sleep > info.next_frame {
                        // retain sleep
                        return Some(sleep);
                    } else if sleep.0 > info.now {
                        // sync-up to frame rate after sleep
                        anim.reset_sleep();
                        return Some(info.next_frame);
                    }
                }

                anim.reset_state(info.animations_enabled, info.now, info.time_scale);

                VARS_ANIMATION_CTRL_CTX.with_context(&mut controller, || {
                    VARS_MODIFY_CTX.with_context(&mut anim_modify_info, || animation(&anim))
                });

                // retain until next frame
                //
                // stop or sleep may be requested after this (during modify apply),
                // these updates are applied on the next frame.
                Some(info.next_frame)
            } else {
                // drop
                controller.as_ref().unwrap().on_stop(&anim);
                None
            }
        }));

        vars.ans_next_frame = Some(Deadline(DInstant::EPOCH));

        vars.wake_app();

        handle
    }

    fn update_animations_impl(&self, timer: &mut dyn AnimationTimer) {
        let mut vars = VARS_SV.write();
        if let Some(next_frame) = vars.ans_next_frame {
            if timer.elapsed(next_frame) {
                let mut animations = mem::take(vars.ans_animations.get_mut());
                debug_assert!(!animations.is_empty());

                let info = AnimationUpdateInfo {
                    animations_enabled: vars.animations_enabled.get(),
                    time_scale: vars.animation_time_scale.get(),
                    now: timer.now(),
                    next_frame: next_frame + vars.frame_duration.get(),
                };

                let mut min_sleep = Deadline(info.now + Duration::from_secs(60 * 60));

                drop(vars);

                animations.retain_mut(|animate| {
                    if let Some(sleep) = animate(info) {
                        min_sleep = min_sleep.min(sleep);
                        true
                    } else {
                        false
                    }
                });

                let mut vars = VARS_SV.write();

                let self_animations = vars.ans_animations.get_mut();
                if !self_animations.is_empty() {
                    min_sleep = Deadline(info.now);
                }
                animations.append(self_animations);
                *self_animations = animations;

                if !self_animations.is_empty() {
                    vars.ans_next_frame = Some(min_sleep);
                    timer.register(min_sleep);
                } else {
                    vars.ans_next_frame = None;
                }
            }
        }
    }

    fn next_deadline_impl(&self, timer: &mut dyn AnimationTimer) {
        if let Some(next_frame) = VARS_SV.read().ans_next_frame {
            timer.register(next_frame);
        }
    }
}

#[derive(Clone, Copy)]
struct AnimationUpdateInfo {
    animations_enabled: bool,
    now: DInstant,
    time_scale: Factor,
    next_frame: Deadline,
}
