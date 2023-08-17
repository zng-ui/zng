use std::{mem, thread::ThreadId, time::Duration};

use zero_ui_view_api::AnimationsConfig;

use crate::{app::LoopTimer, context::app_local, context_local, units::Factor};

use super::{
    animation::{Animations, ModifyInfo},
    *,
};

/// Represents the last time a variable was mutated or the current update cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::NoUninit)]
#[repr(transparent)]
pub struct VarUpdateId(u32);
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

pub(super) type VarUpdateFn = Box<dyn FnOnce() + Send>;

app_local! {
    pub(crate) static VARS_SV: VarsService = VarsService::new();
}

context_local! {
    pub(crate) static VARS_MODIFY_CTX: Option<ModifyInfo> = None;
}

pub(crate) struct VarsService {
    pub(super) ans: Animations,

    update_id: VarUpdateId,

    updates: Mutex<Vec<(ModifyInfo, VarUpdateFn)>>,
    updating_thread: Option<ThreadId>,
    updates_after: Mutex<Vec<(ModifyInfo, VarUpdateFn)>>,

    modify_receivers: Mutex<Vec<Box<dyn Fn() -> bool + Send>>>,
}
impl VarsService {
    pub(crate) fn new() -> Self {
        Self {
            ans: Animations::new(),
            update_id: VarUpdateId(1),
            updates: Mutex::new(vec![]),
            updating_thread: None,
            updates_after: Mutex::new(vec![]),
            modify_receivers: Mutex::new(vec![]),
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

    /// Returns a read-only variable that tracks if animations are enabled in the operating system.
    ///
    /// If `false` all animations must be skipped to the end, users with photo-sensitive epilepsy disable animations system wide.
    pub fn animations_enabled(&self) -> ReadOnlyArcVar<bool> {
        VARS_SV.read().ans.animations_enabled.read_only()
    }

    /// Variable that defines the global frame duration, the default is 60fps `(1.0 / 60.0).secs()`.
    pub fn frame_duration(&self) -> ArcVar<Duration> {
        VARS_SV.read().ans.frame_duration.clone()
    }

    /// Variable that defines a global scale for the elapsed time of animations.
    pub fn animation_time_scale(&self) -> ArcVar<Factor> {
        VARS_SV.read().ans.animation_time_scale.clone()
    }

    /// Info about the current context when requesting variable modification.
    ///
    /// If is current inside a [`VARS.animate`] closure, or inside a [`Var::modify`] closure requested by an animation, or inside
    /// an [`AnimationController`], returns the info that was collected at the moment the animation was requested. Outside of animations
    /// gets an info with [`importance`] guaranteed to override the [`modify_importance`].
    ///
    /// [`importance`]: ModifyInfo::importance
    /// [`modify_importance`]: AnyVar::modify_importance
    /// [`AnimationController`]: animation::AnimationController
    pub fn current_modify(&self) -> ModifyInfo {
        match VARS_MODIFY_CTX.get_clone() {
            Some(current) => current, // override set by modify and animation closures.
            None => VARS_SV.read().ans.current_modify.clone(),
        }
    }

    /// Adds an animation handler that is called every frame to update captured variables.
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
    /// # use zero_ui_core::{var::*, *, units::*, text::*, handler::*};
    /// #
    /// fn animate_text(text: &impl Var<Txt>, completed: &impl Var<bool>) {
    ///     let transition = animation::Transition::new(0u8, 100);
    ///     let mut prev_value = 101;
    ///     VARS.animate(clmv!(text, completed, |animation| {
    ///         let step = easing::expo(animation.elapsed_stop(1.secs()));
    ///         let value = transition.sample(step);
    ///         if value != prev_value {
    ///             if value == 100 {
    ///                 animation.stop();
    ///                 completed.set(true);
    ///             }
    ///             let _ = text.set(formatx!("Animation at {value}%"));
    ///             prev_value = value;
    ///         }
    ///     }))
    ///     .perm()
    /// }
    /// ```
    ///
    /// Note that the animation can be stopped from the inside, the closure second parameter is an [`Animation`]. In
    /// the example this is the only way to stop the animation, because we called [`perm`]. Animations hold a clone
    /// of the variables they affect and exist for the duration of the app if not stopped, causing the app to wake and call the
    /// animation closure for every frame.
    ///
    /// This method is the most basic animation interface, used to build all other animations and *easing*, its rare that you
    /// will need to use it directly, most of the time animation effects can be composted using the [`Var`] easing and mapping
    /// methods.
    ///
    /// ```
    /// # use zero_ui_core::{var::*, *, units::*, text::*, handler::*};
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
    /// Animations have their control over a variable permanently overridden when a newer animation modifies it or
    /// it is modified directly, but even if overridden **the animation keeps running**. This happens because the system has no insight of
    /// all side effects caused by the `animation` closure. You can use the [`VARS.current_modify`] and [`AnyVar::modify_importance`]
    /// to detect when the animation no longer affects any variables and stop it.
    ///
    /// These optimizations are implemented by the animations provided as methods of [`Var<T>`].
    ///
    /// # External Controller
    ///
    /// The animation can be controlled from the inside using the [`Animation`] reference, it can be stopped using the returned
    /// [`AnimationHandle`], and it can also be controlled by a registered [`AnimationController`] that can manage multiple
    /// animations at the same time, see [`with_animation_controller`] for more details.
    ///
    /// [`AnimationHandle`]: animation::AnimationHandle
    /// [`AnimationController`]: animation::AnimationController
    /// [`Animation`]: animation::Animation
    /// [`Animation::sleep`]: animation::Animation::sleep
    /// [`stop`]: animation::AnimationHandle::stop
    /// [`perm`]: animation::AnimationHandle::perm
    /// [`with_animation_controller`]: Self::with_animation_controller
    pub fn animate<A>(&self, animation: A) -> animation::AnimationHandle
    where
        A: FnMut(&animation::Animation) + Send + 'static,
    {
        Animations::animate(animation)
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
    /// use this method and the [`NilAnimationObserver`] to avoid this behavior.
    ///
    /// [`Animation`]: animation::Animation
    /// [`NilAnimationObserver`]: animation::NilAnimationObserver
    pub fn with_animation_controller<R>(&self, controller: impl animation::AnimationController, animate: impl FnOnce() -> R) -> R {
        animation::VARS_ANIMATION_CTRL_CTX.with_context_value(Box::new(controller), animate)
    }

    pub(super) fn schedule_update(&self, update: VarUpdateFn) {
        let vars = VARS_SV.read();
        let curr_modify = match VARS_MODIFY_CTX.get_clone() {
            Some(current) => current, // override set by modify and animation closures.
            None => vars.ans.current_modify.clone(),
        };

        if let Some(id) = vars.updating_thread {
            if std::thread::current().id() == id {
                // is binding request, enqueue for immediate exec.
                vars.updates.lock().push((curr_modify, update));
            } else {
                // is request from app task thread when we are already updating, enqueue for exec after current update.
                vars.updates_after.lock().push((curr_modify, update));
            }
        } else {
            // request from any app thread,
            vars.updates.lock().push((curr_modify, update));
            UPDATES.send_awake();
        }
    }

    pub(crate) fn apply_updates(&self) {
        let _s = tracing::trace_span!("VARS").entered();
        Self::apply_updates_and_after(0)
    }
    fn apply_updates_and_after(depth: u8) {
        let mut vars = VARS_SV.write();

        match depth {
            0 => {
                vars.update_id.next();
                vars.ans.animation_start_time = None;
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
                Self::apply_updates_and_after(depth + 1)
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

            for (info, update) in updates {
                VARS_MODIFY_CTX.with_context_value(Some(info), update);

                let mut vars = VARS_SV.write();
                let updates = mem::take(vars.updates.get_mut());
                if !updates.is_empty() {
                    drop(vars);
                    update_each_and_bindings(updates, depth + 1);
                }
            }
        }
    }

    pub(crate) fn register_channel_recv(&self, recv_modify: Box<dyn Fn() -> bool + Send>) {
        VARS_SV.read().modify_receivers.lock().push(recv_modify);
    }

    pub(crate) fn receive_sended_modify(&self) {
        let mut rcvs = mem::take(&mut *VARS_SV.read().modify_receivers.lock());
        rcvs.retain(|rcv| rcv());

        let mut vars = VARS_SV.write();
        rcvs.append(vars.modify_receivers.get_mut());
        *vars.modify_receivers.get_mut() = rcvs;
    }

    pub(crate) fn update_animations_config(&self, cfg: &AnimationsConfig) {
        VARS_SV.read().ans.animations_enabled.set(cfg.enabled);
    }

    /// Called in `update_timers`, does one animation frame if the frame duration has elapsed.
    pub(crate) fn update_animations(&self, timer: &mut LoopTimer) {
        Animations::update_animations(timer)
    }

    /// Returns the next animation frame, if there are any active animations.
    pub(crate) fn next_deadline(&self, timer: &mut LoopTimer) {
        Animations::next_deadline(timer)
    }

    pub(crate) fn has_pending_updates(&self) -> bool {
        !VARS_SV.write().updates.get_mut().is_empty()
    }
}
