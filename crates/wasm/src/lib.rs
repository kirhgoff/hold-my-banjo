use hold_my_banjo_core::mix::Bonk;
use hold_my_banjo_core::sim::{Mode, Scenario};
use hold_my_banjo_core::Chorus;

pub struct Engine {
    chorus: Chorus,
    out: Vec<f32>,
    fired: Vec<u32>,
    positions: Vec<f64>,
}

fn mode(id: u32) -> Mode {
    match id {
        1 => Mode::EventDelay,
        2 => Mode::EventAdvance,
        3 => Mode::PhaseCoupled,
        _ => Mode::Independent,
    }
}

fn apply_params(
    s: &mut Scenario,
    frog_count: u32,
    mode_id: u32,
    strength: f64,
    mean_interval_s: f64,
    hearing_radius_m: f64,
) {
    s.frog_count = (frog_count as usize).clamp(1, 100);
    s.behaviour.mode = mode(mode_id);
    s.behaviour.coupling_strength = strength.clamp(0.0, 1.0);
    s.behaviour.mean_interval_s =
        mean_interval_s.clamp(s.behaviour.min_interval_s, s.behaviour.max_interval_s);
    s.behaviour.hearing_radius_m = hearing_radius_m.max(0.1);
}

#[unsafe(no_mangle)]
pub extern "C" fn hmb_new(
    sample_rate: u32,
    seed: u32,
    frog_count: u32,
    mode_id: u32,
    strength: f64,
    mean_interval_s: f64,
    hearing_radius_m: f64,
) -> *mut Engine {
    let mut s = Scenario::bundled();
    s.seed = seed as u64;
    apply_params(
        &mut s,
        frog_count,
        mode_id,
        strength,
        mean_interval_s,
        hearing_radius_m,
    );
    let chorus = Chorus::new(s, sample_rate);
    let engine = Engine {
        chorus,
        out: Vec::new(),
        fired: Vec::new(),
        positions: Vec::new(),
    };
    Box::into_raw(Box::new(engine))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_free(e: *mut Engine) {
    drop(Box::from_raw(e));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_render(e: *mut Engine, frames: u32) -> *const f32 {
    let engine = &mut *e;
    engine.out.resize(frames as usize * 2, 0.0);
    let events = engine.chorus.render(&mut engine.out);
    engine.fired = events.iter().map(|ev| ev.frog_id as u32).collect();
    engine.out.as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_event_count(e: *mut Engine) -> u32 {
    (&*e).fired.len() as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_events(e: *mut Engine) -> *const u32 {
    (&*e).fired.as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_frog_count(e: *mut Engine) -> u32 {
    (&*e).chorus.sim().scenario().frog_count as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_positions(e: *mut Engine) -> *const f64 {
    let engine = &mut *e;
    engine.positions = engine
        .chorus
        .sim()
        .positions()
        .into_iter()
        .flat_map(|(x, y)| [x, y])
        .collect();
    engine.positions.as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_scene_width(e: *mut Engine) -> f64 {
    (&*e).chorus.sim().scenario().scene.width_m
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_scene_depth(e: *mut Engine) -> f64 {
    (&*e).chorus.sim().scenario().scene.depth_m
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_set_frog_count(e: *mut Engine, n: u32) {
    let engine = &mut *e;
    engine
        .chorus
        .sim_mut()
        .update(|s| s.frog_count = (n as usize).clamp(1, 100));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_set_mode(e: *mut Engine, id: u32) {
    let engine = &mut *e;
    engine
        .chorus
        .sim_mut()
        .update(|s| s.behaviour.mode = mode(id));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_set_strength(e: *mut Engine, k: f64) {
    let engine = &mut *e;
    engine
        .chorus
        .sim_mut()
        .update(|s| s.behaviour.coupling_strength = k.clamp(0.0, 1.0));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_set_mean_interval(e: *mut Engine, v: f64) {
    let engine = &mut *e;
    engine.chorus.sim_mut().update(|s| {
        s.behaviour.mean_interval_s =
            v.clamp(s.behaviour.min_interval_s, s.behaviour.max_interval_s)
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_set_hearing_radius(e: *mut Engine, v: f64) {
    let engine = &mut *e;
    engine
        .chorus
        .sim_mut()
        .update(|s| s.behaviour.hearing_radius_m = v.max(0.1));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_set_bonk(
    e: *mut Engine,
    pitch_hz: f64,
    spread_hz: f64,
    duration_s: f64,
    attack_s: f64,
    decay_s: f64,
    sweep_depth: f64,
    sweep_s: f64,
    harmonic2: f64,
    harmonic3: f64,
    subharmonic: f64,
) {
    let b = Bonk {
        pitch_hz: pitch_hz.clamp(100.0, 1500.0),
        spread_hz: spread_hz.clamp(0.0, 200.0),
        duration_s: duration_s.clamp(0.05, 1.0),
        attack_s: attack_s.clamp(0.0005, 0.1),
        decay_s: decay_s.clamp(0.005, 1.0),
        sweep_depth: sweep_depth.clamp(-0.5, 1.0),
        sweep_s: sweep_s.clamp(0.001, 0.2),
        harmonic2: harmonic2.clamp(0.0, 1.0),
        harmonic3: harmonic3.clamp(0.0, 1.0),
        subharmonic: subharmonic.clamp(0.0, 1.0),
    };
    (&mut *e).chorus.set_bonk(&b);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_audition(e: *mut Engine) {
    (&mut *e).chorus.audition();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_bonk_len(e: *mut Engine) -> u32 {
    (&*e).chorus.clip(1).len() as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hmb_bonk_samples(e: *mut Engine) -> *const f32 {
    (&*e).chorus.clip(1).as_ptr()
}
