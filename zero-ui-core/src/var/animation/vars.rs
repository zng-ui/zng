use std::{
    cell::{Cell, RefCell},
    mem,
};

use crate::{
    app::LoopTimer,
    crate_util::RunOnDrop,
    units::*,
    var::{var, RcVar, VarsRead},
};

use super::*;

type AnimationFn = Box<dyn FnMut(&Vars, AnimationUpdateInfo) -> Option<Deadline>>;

#[derive(Clone, Copy)]
struct AnimationUpdateInfo {
    animations_enabled: bool,
    now: Instant,
    time_scale: Factor,
    next_frame: Deadline,
}

/// Part of `Vars` that controls animations.
pub(crate) struct VarsAnimations {
    animations: RefCell<Vec<AnimationFn>>,
    animation_id: Cell<u32>,
    pub(crate) current_animation: RefCell<(Option<WeakAnimationHandle>, u32)>,
    pub(crate) animation_start_time: Cell<Option<Instant>>,
    next_frame: Cell<Option<Deadline>>,
    pub(crate) animations_enabled: RcVar<bool>,
    pub(crate) frame_duration: RcVar<Duration>,
    pub(crate) animation_time_scale: RcVar<Factor>,
}
impl VarsAnimations {
    pub(crate) fn new() -> Self {
        Self {
            animations: RefCell::default(),
            animation_id: Cell::new(1),
            current_animation: RefCell::new((None, 1)),
            animation_start_time: Cell::new(None),
            next_frame: Cell::new(None),
            animations_enabled: var(true),
            frame_duration: var((1.0 / 60.0).secs()),
            animation_time_scale: var(1.fct()),
        }
    }

    pub(crate) fn update_animations(vars: &mut Vars, timer: &mut LoopTimer) {
        if let Some(next_frame) = vars.ans.next_frame.get() {
            if timer.elapsed(next_frame) {
                let mut animations = mem::take(&mut *vars.ans.animations.borrow_mut());
                debug_assert!(!animations.is_empty());

                let info = AnimationUpdateInfo {
                    animations_enabled: vars.ans.animations_enabled.copy(vars),
                    time_scale: vars.ans.animation_time_scale.copy(vars),
                    now: Instant::now(),
                    next_frame: next_frame + vars.ans.frame_duration.copy(vars),
                };

                let mut min_sleep = Deadline(info.now + Duration::from_secs(60 * 60));

                animations.retain_mut(|animate| {
                    if let Some(sleep) = animate(vars, info) {
                        min_sleep = min_sleep.min(sleep);
                        true
                    } else {
                        false
                    }
                });

                let mut self_animations = vars.ans.animations.borrow_mut();
                animations.extend(self_animations.drain(..));
                *self_animations = animations;

                if !self_animations.is_empty() {
                    vars.ans.next_frame.set(Some(min_sleep));
                    timer.register(min_sleep);
                } else {
                    vars.ans.next_frame.set(None);
                }
            }
        }
    }

    pub(crate) fn next_deadline(vars: &mut Vars, timer: &mut LoopTimer) {
        if let Some(next_frame) = vars.ans.next_frame.get() {
            timer.register(next_frame);
        }
    }

    pub(crate) fn animate<A>(vars: &VarsRead, mut animation: A) -> AnimationHandle
    where
        A: FnMut(&Vars, &AnimationArgs) + 'static,
    {
        // # Animation ID
        //
        // Variables only accept modifications from an animation ID >= the previous animation ID that modified it.
        //
        // Direct modifications always overwrite previous animations, so we advance the ID for each call to
        // this method **and then** advance the ID again for all subsequent direct modifications.
        //
        // Example sequence of events:
        //
        // |ID| Modification  | Accepted
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
        let start_time = if let Some(t) = vars.ans.animation_start_time.get() {
            t
        } else {
            let t = Instant::now();
            vars.ans.animation_start_time.set(Some(t));
            t
        };

        let mut id = vars.ans.animation_id.get().wrapping_add(1);
        if id == 0 {
            id = 1;
        }
        let mut next_set_id = id.wrapping_add(1);
        if next_set_id == 0 {
            next_set_id = 1;
        }
        vars.ans.animation_id.set(next_set_id);
        vars.ans.current_animation.borrow_mut().1 = next_set_id;

        let handle_owner;
        let handle;
        let weak_handle;

        if let Some(parent_handle) = vars.ans.current_animation.borrow().0.clone() {
            // is `animate` request inside other animate closure,
            // in this case we give it the same animation handle as the *parent*
            // animation, that holds the actual handle owner.
            handle_owner = None;

            if let Some(h) = parent_handle.upgrade() {
                handle = h;
            } else {
                // attempt to create new animation from inside dropping animation, ignore
                return AnimationHandle::dummy();
            }

            weak_handle = parent_handle;
        } else {
            let (o, h) = AnimationHandle::new();
            handle_owner = Some(o);
            weak_handle = h.downgrade();
            handle = h;
        };

        let mut anim = AnimationArgs::new(
            vars.ans.animations_enabled.copy(vars),
            start_time,
            vars.ans.animation_time_scale.copy(vars),
        );
        vars.ans.animations.borrow_mut().push(Box::new(move |vars, info| {
            let _handle_owner = &handle_owner; // capture and own the handle owner.

            if weak_handle.upgrade().is_some() {
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

                let prev = mem::replace(&mut *vars.ans.current_animation.borrow_mut(), (Some(weak_handle.clone()), id));
                let _cleanup = RunOnDrop::new(|| *vars.ans.current_animation.borrow_mut() = prev);

                animation(vars, &anim);

                if anim.stop_requested() {
                    // drop
                    return None;
                }

                // retain
                match anim.sleep_deadline() {
                    Some(sleep) if sleep > info.next_frame => Some(sleep),
                    _ => Some(info.next_frame),
                }
            } else {
                // drop
                None
            }
        }));

        if vars.ans.next_frame.get().is_none() {
            vars.ans.next_frame.set(Some(Deadline(Instant::now())));
        }

        handle
    }
}
