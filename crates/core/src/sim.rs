use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::Deserialize;
use std::f64::consts::{PI, TAU};
use std::path::PathBuf;

pub const DT: f64 = 0.001;

#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Independent,
    EventDelay,
    EventAdvance,
    PhaseCoupled,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Independent => "independent",
            Mode::EventDelay => "event_delay",
            Mode::EventAdvance => "event_advance",
            Mode::PhaseCoupled => "phase_coupled",
        }
    }

    pub fn hypothetical(self) -> bool {
        self != Mode::Independent
    }
}

impl std::str::FromStr for Mode {
    type Err = String;
    fn from_str(s: &str) -> Result<Mode, String> {
        match s {
            "independent" => Ok(Mode::Independent),
            "event_delay" => Ok(Mode::EventDelay),
            "event_advance" => Ok(Mode::EventAdvance),
            "phase_coupled" => Ok(Mode::PhaseCoupled),
            _ => Err(format!("unknown mode {s}")),
        }
    }
}

#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    Measured,
    Experimental,
}

impl Provenance {
    pub fn name(self) -> &'static str {
        match self {
            Provenance::Measured => "measured",
            Provenance::Experimental => "experimental",
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub schema_version: u32,
    pub provenance: Provenance,
    pub seed: u64,
    pub duration_s: f64,
    pub frog_count: usize,
    pub behaviour: Behaviour,
    pub scene: Scene,
    pub audio: Audio,
    pub output: Output,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Behaviour {
    pub mode: Mode,
    pub mean_interval_s: f64,
    pub relative_interval_sd: f64,
    pub min_interval_s: f64,
    pub max_interval_s: f64,
    pub min_retrigger_s: f64,
    pub coupling_strength: f64,
    pub hearing_radius_m: f64,
    pub hearing_decay_m: f64,
    pub response_cooldown_s: f64,
    pub phase_noise: f64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub width_m: f64,
    pub depth_m: f64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Audio {
    pub asset_dir: PathBuf,
    pub preferred_sample_rate: u32,
    pub master_gain: f32,
    pub stereo_width: f64,
    pub pitch_sd_semitones: f64,
    pub gain_sd_db: f64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub event_csv: Option<PathBuf>,
    pub offline_wav: Option<PathBuf>,
}

impl Scenario {
    pub fn parse(text: &str) -> Result<Scenario, toml::de::Error> {
        toml::from_str(text)
    }

    pub fn bundled() -> Scenario {
        Scenario::parse(include_str!("../../../scenarios/default.toml")).unwrap()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "schema_version must be 1, got {}",
                self.schema_version
            ));
        }
        if self.frog_count < 1 || self.frog_count > 100 {
            return Err(format!(
                "frog_count must be between 1 and 100, got {}",
                self.frog_count
            ));
        }
        if !self.duration_s.is_finite() || self.duration_s <= 0.0 {
            return Err(format!(
                "duration_s must be finite and > 0, got {}",
                self.duration_s
            ));
        }

        let b = &self.behaviour;
        if !(10.0 * DT <= b.min_interval_s
            && b.min_interval_s <= b.mean_interval_s
            && b.mean_interval_s <= b.max_interval_s)
        {
            return Err(format!(
                "behaviour intervals must satisfy {} <= min_interval_s <= mean_interval_s <= max_interval_s, got min={} mean={} max={}",
                10.0 * DT,
                b.min_interval_s,
                b.mean_interval_s,
                b.max_interval_s
            ));
        }
        if b.relative_interval_sd < 0.0 {
            return Err(format!(
                "behaviour.relative_interval_sd must be >= 0, got {}",
                b.relative_interval_sd
            ));
        }
        if b.min_retrigger_s < 0.0 {
            return Err(format!(
                "behaviour.min_retrigger_s must be >= 0, got {}",
                b.min_retrigger_s
            ));
        }
        if b.response_cooldown_s < 0.0 {
            return Err(format!(
                "behaviour.response_cooldown_s must be >= 0, got {}",
                b.response_cooldown_s
            ));
        }
        if b.phase_noise < 0.0 {
            return Err(format!(
                "behaviour.phase_noise must be >= 0, got {}",
                b.phase_noise
            ));
        }
        if b.coupling_strength < 0.0 {
            return Err(format!(
                "behaviour.coupling_strength must be >= 0, got {}",
                b.coupling_strength
            ));
        }
        match b.mode {
            Mode::EventDelay | Mode::EventAdvance => {
                if b.coupling_strength > 1.0 {
                    return Err(format!(
                        "behaviour.coupling_strength must be <= 1 in mode {}, got {}",
                        b.mode.name(),
                        b.coupling_strength
                    ));
                }
            }
            Mode::PhaseCoupled => {
                let limit = b.coupling_strength * self.frog_count as f64 * DT;
                if limit >= 0.5 {
                    return Err(format!(
                        "behaviour.coupling_strength * frog_count * DT must be < 0.5 in phase_coupled mode, got {limit}"
                    ));
                }
            }
            Mode::Independent => {}
        }
        if b.hearing_radius_m <= 0.0 {
            return Err(format!(
                "behaviour.hearing_radius_m must be > 0, got {}",
                b.hearing_radius_m
            ));
        }
        if b.hearing_decay_m <= 0.0 {
            return Err(format!(
                "behaviour.hearing_decay_m must be > 0, got {}",
                b.hearing_decay_m
            ));
        }
        if self.scene.width_m <= 0.0 {
            return Err(format!(
                "scene.width_m must be > 0, got {}",
                self.scene.width_m
            ));
        }
        if self.scene.depth_m <= 0.0 {
            return Err(format!(
                "scene.depth_m must be > 0, got {}",
                self.scene.depth_m
            ));
        }
        if !(self.audio.master_gain > 0.0 && self.audio.master_gain <= 4.0) {
            return Err(format!(
                "audio.master_gain must be in (0, 4], got {}",
                self.audio.master_gain
            ));
        }
        if !(0.0..=1.0).contains(&self.audio.stereo_width) {
            return Err(format!(
                "audio.stereo_width must be in [0, 1], got {}",
                self.audio.stereo_width
            ));
        }
        if self.audio.pitch_sd_semitones < 0.0 {
            return Err(format!(
                "audio.pitch_sd_semitones must be >= 0, got {}",
                self.audio.pitch_sd_semitones
            ));
        }
        if self.audio.gain_sd_db < 0.0 {
            return Err(format!(
                "audio.gain_sd_db must be >= 0, got {}",
                self.audio.gain_sd_db
            ));
        }
        if !(8000..=192000).contains(&self.audio.preferred_sample_rate) {
            return Err(format!(
                "audio.preferred_sample_rate must be between 8000 and 192000, got {}",
                self.audio.preferred_sample_rate
            ));
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CallEvent {
    pub time_s: f64,
    pub frog_id: usize,
    pub x_m: f64,
    pub y_m: f64,
    pub clip_id: usize,
    pub pitch_ratio: f64,
    pub gain: f64,
}

struct Frog {
    id: usize,
    x: f64,
    y: f64,
    phase: f64,
    interval_jitter: f64,
    intrinsic_s: f64,
    period_s: f64,
    next_allowed_s: f64,
    rng: ChaCha8Rng,
}

struct Link {
    frog: usize,
    weight: f64,
    ready_at: f64,
}

fn normal(rng: &mut ChaCha8Rng) -> f64 {
    let u1 = 1.0 - rng.gen::<f64>();
    let u2 = rng.gen::<f64>();
    (-2.0 * u1.ln()).sqrt() * (TAU * u2).cos()
}

impl Frog {
    fn new(id: usize, s: &Scenario) -> Frog {
        let mut rng = ChaCha8Rng::seed_from_u64(s.seed);
        rng.set_stream(id as u64);

        let half_w = s.scene.width_m / 2.0;
        let x = rng.gen_range(-half_w..=half_w);
        let y = rng.gen_range(0.0..=s.scene.depth_m);
        let phase = rng.gen::<f64>();

        let b = &s.behaviour;
        let interval_jitter = 1.0 + b.relative_interval_sd * normal(&mut rng);
        let intrinsic_s =
            (b.mean_interval_s * interval_jitter).clamp(b.min_interval_s, b.max_interval_s);

        let mut frog = Frog {
            id,
            x,
            y,
            phase,
            interval_jitter,
            intrinsic_s,
            period_s: intrinsic_s,
            next_allowed_s: 0.0,
            rng,
        };
        frog.period_s = frog.sample_interval(s);
        frog
    }

    fn sample_interval(&mut self, s: &Scenario) -> f64 {
        let b = &s.behaviour;
        (self.intrinsic_s * (1.0 + b.relative_interval_sd * normal(&mut self.rng)))
            .clamp(b.min_interval_s, b.max_interval_s)
    }

    fn call(&mut self, now: f64, s: &Scenario, clip_count: usize) -> CallEvent {
        self.phase = self.phase.fract();
        self.next_allowed_s = now + s.behaviour.min_retrigger_s;
        let clip_id = self.rng.gen_range(0..clip_count);
        let pitch_ratio = 2f64.powf(s.audio.pitch_sd_semitones * normal(&mut self.rng) / 12.0);
        let gain = 10f64.powf(s.audio.gain_sd_db * normal(&mut self.rng) / 20.0);
        self.period_s = self.sample_interval(s);

        CallEvent {
            time_s: now,
            frog_id: self.id,
            x_m: self.x,
            y_m: self.y,
            clip_id,
            pitch_ratio,
            gain,
        }
    }
}

fn build_links(frogs: &[Frog], s: &Scenario) -> Vec<Vec<Link>> {
    let n = frogs.len();
    let mut links: Vec<Vec<Link>> = (0..n).map(|_| Vec::new()).collect();
    for a in 0..n {
        for b in 0..n {
            if a == b {
                continue;
            }
            let dx = frogs[a].x - frogs[b].x;
            let dy = frogs[a].y - frogs[b].y;
            let d = dx.hypot(dy);
            if d <= s.behaviour.hearing_radius_m {
                let weight = (-d / s.behaviour.hearing_decay_m).exp();
                links[a].push(Link {
                    frog: b,
                    weight,
                    ready_at: 0.0,
                });
            }
        }
    }
    links
}

pub struct Sim {
    s: Scenario,
    clip_count: usize,
    frogs: Vec<Frog>,
    links: Vec<Vec<Link>>,
    drift: Vec<f64>,
    step: u64,
}

impl Sim {
    pub fn new(s: Scenario, clip_count: usize) -> Sim {
        let frogs: Vec<Frog> = (0..s.frog_count).map(|id| Frog::new(id, &s)).collect();
        let links = build_links(&frogs, &s);
        let drift = vec![0.0; frogs.len()];
        Sim {
            s,
            clip_count,
            frogs,
            links,
            drift,
            step: 0,
        }
    }

    pub fn scenario(&self) -> &Scenario {
        &self.s
    }

    pub fn time_s(&self) -> f64 {
        self.step as f64 * DT
    }

    pub fn positions(&self) -> Vec<(f64, f64)> {
        self.frogs.iter().map(|f| (f.x, f.y)).collect()
    }

    pub fn update(&mut self, change: impl FnOnce(&mut Scenario)) {
        change(&mut self.s);
        let b = &self.s.behaviour;
        for f in &mut self.frogs {
            f.intrinsic_s =
                (b.mean_interval_s * f.interval_jitter).clamp(b.min_interval_s, b.max_interval_s);
        }
        self.frogs.truncate(self.s.frog_count);
        for id in self.frogs.len()..self.s.frog_count {
            self.frogs.push(Frog::new(id, &self.s));
        }
        self.links = build_links(&self.frogs, &self.s);
        self.drift = vec![0.0; self.frogs.len()];
    }

    pub fn step(&mut self, events: &mut Vec<CallEvent>) {
        let Sim {
            s,
            clip_count,
            frogs,
            links,
            drift,
            step,
        } = self;
        *step += 1;
        let now = *step as f64 * DT;
        let n = frogs.len();
        let k = s.behaviour.coupling_strength;
        let mode = s.behaviour.mode;

        if mode == Mode::PhaseCoupled {
            for i in 0..n {
                let phase_i = frogs[i].phase;
                let sum: f64 = links[i]
                    .iter()
                    .map(|l| l.weight * (TAU * (frogs[l.frog].phase - phase_i)).sin())
                    .sum();
                drift[i] = -k * DT * sum;
            }
        }

        let first_new = events.len();

        for i in 0..n {
            let d = drift[i];
            let f = &mut frogs[i];
            f.phase += DT / f.period_s + d;
            if mode == Mode::PhaseCoupled && s.behaviour.phase_noise > 0.0 {
                f.phase += s.behaviour.phase_noise * DT.sqrt() * normal(&mut f.rng);
            }
            if f.phase >= 1.0 {
                if now >= f.next_allowed_s {
                    events.push(f.call(now, s, *clip_count));
                } else {
                    f.phase = 1.0;
                }
            }
        }

        if matches!(mode, Mode::EventDelay | Mode::EventAdvance) && k > 0.0 {
            let sign = if mode == Mode::EventDelay { -1.0 } else { 1.0 };
            for e in &events[first_new..] {
                for link in links[e.frog_id].iter_mut() {
                    if now >= link.ready_at {
                        let weight = link.weight;
                        let p = &mut frogs[link.frog].phase;
                        *p = (*p + sign * k * weight * (PI * *p).sin()).clamp(0.0, 0.999);
                        link.ready_at = now + s.behaviour.response_cooldown_s;
                    }
                }
            }
        }
    }
}

pub fn simulate(s: &Scenario, clip_count: usize) -> Vec<CallEvent> {
    let mut sim = Sim::new(s.clone(), clip_count);
    let steps = (s.duration_s / DT).round() as u64;
    let mut events = Vec::new();
    for _ in 0..steps {
        sim.step(&mut events);
    }
    events
}

pub fn events_csv(s: &Scenario, clip_source: &str, events: &[CallEvent]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# provenance={}\n", s.provenance.name()));
    out.push_str(&format!(
        "# mode={} hypothetical={}\n",
        s.behaviour.mode.name(),
        s.behaviour.mode.hypothetical()
    ));
    out.push_str(&format!(
        "# schema_version={} seed={} frogs={} clips={}\n",
        s.schema_version, s.seed, s.frog_count, clip_source
    ));
    out.push_str("time_s,frog_id,x_m,y_m,clip_id,pitch_ratio,gain\n");
    for e in events {
        out.push_str(&format!(
            "{:.6},{},{:.3},{:.3},{},{:.6},{:.6}\n",
            e.time_s, e.frog_id, e.x_m, e.y_m, e.clip_id, e.pitch_ratio, e.gain
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_scenario() -> Scenario {
        Scenario::bundled()
    }

    #[test]
    fn ac01_frog_count_bounds() {
        let mut s = base_scenario();
        s.frog_count = 0;
        assert!(s.validate().is_err());

        s.frog_count = 101;
        assert!(s.validate().is_err());

        s.frog_count = 1;
        s.duration_s = 20.0;
        assert!(s.validate().is_ok());
        let events = simulate(&s, 1);
        assert!(events.len() >= 2);
    }

    #[test]
    fn ac02_csv_identical() {
        let s = base_scenario();
        let events_a = simulate(&s, 3);
        let events_b = simulate(&s, 3);
        let csv_a = events_csv(&s, "wav", &events_a);
        let csv_b = events_csv(&s, "wav", &events_b);
        assert_eq!(csv_a, csv_b);
    }

    #[test]
    fn ac03_bounds_and_seed() {
        let mut s = base_scenario();
        s.duration_s = 60.0;

        let mut s1 = s.clone();
        s1.seed = 1;
        let mut s2 = s.clone();
        s2.seed = 2;

        let events1 = simulate(&s1, 3);
        let events2 = simulate(&s2, 3);
        let frog0_1: Vec<f64> = events1
            .iter()
            .filter(|e| e.frog_id == 0)
            .map(|e| e.time_s)
            .collect();
        let frog0_2: Vec<f64> = events2
            .iter()
            .filter(|e| e.frog_id == 0)
            .map(|e| e.time_s)
            .collect();
        assert_ne!(frog0_1, frog0_2);

        s.seed = 7;
        let events = simulate(&s, 3);
        let b = &s.behaviour;
        for frog_id in 0..s.frog_count {
            let times: Vec<f64> = events
                .iter()
                .filter(|e| e.frog_id == frog_id)
                .map(|e| e.time_s)
                .collect();
            for w in times.windows(2) {
                let ioi = w[1] - w[0];
                assert!(ioi >= b.min_interval_s - 2.0 * DT);
                assert!(ioi <= b.max_interval_s + 2.0 * DT);
                assert!(ioi >= b.min_retrigger_s);
            }
        }
        for e in &events {
            assert!(e.pitch_ratio.is_finite() && e.pitch_ratio > 0.0);
            assert!(e.gain.is_finite() && e.gain > 0.0);
        }
    }

    #[test]
    fn ac04_independent_isolated() {
        let mut s = base_scenario();
        s.duration_s = 30.0;

        let mut s1 = s.clone();
        s1.frog_count = 1;
        let mut s2 = s.clone();
        s2.frog_count = 2;
        s.frog_count = 5;

        let frog0_events = |events: Vec<CallEvent>| -> Vec<CallEvent> {
            events.into_iter().filter(|e| e.frog_id == 0).collect()
        };

        let e1 = frog0_events(simulate(&s1, 3));
        let e2 = frog0_events(simulate(&s2, 3));
        let e5 = frog0_events(simulate(&s, 3));

        assert_eq!(e1, e2);
        assert_eq!(e1, e5);
    }

    #[test]
    fn ac05_zero_strength_is_baseline() {
        let mut baseline = base_scenario();
        baseline.duration_s = 30.0;
        let independent_events = simulate(&baseline, 3);

        for mode in [Mode::EventDelay, Mode::EventAdvance, Mode::PhaseCoupled] {
            let mut s = baseline.clone();
            s.behaviour.mode = mode;
            s.behaviour.coupling_strength = 0.0;
            let events = simulate(&s, 3);
            assert_eq!(
                events, independent_events,
                "mode {:?} should match independent baseline at K=0",
                mode
            );
        }
    }

    fn circ_dist(a: f64, b: f64) -> f64 {
        let d = (a - b).rem_euclid(1.0);
        d.min(1.0 - d)
    }

    fn final_pair_delta(events: &[CallEvent], mean_interval_s: f64) -> f64 {
        let t0: Vec<f64> = events
            .iter()
            .filter(|e| e.frog_id == 0)
            .map(|e| e.time_s)
            .collect();
        let t1: Vec<f64> = events
            .iter()
            .filter(|e| e.frog_id == 1)
            .map(|e| e.time_s)
            .collect();
        let last_t1 = *t1.last().expect("frog 1 should have called");
        let prev_t0 = t0
            .iter()
            .copied()
            .filter(|&t| t < last_t1)
            .last()
            .expect("frog 0 should have called before frog 1's last call");
        ((last_t1 - prev_t0) / mean_interval_s).rem_euclid(1.0)
    }

    #[test]
    fn ac05_positive_strength_changes_pair_phase() {
        let mut base = base_scenario();
        base.seed = 42017;
        base.frog_count = 2;
        base.scene.width_m = 2.0;
        base.scene.depth_m = 2.0;
        base.behaviour.relative_interval_sd = 0.0;
        base.duration_s = 120.0;

        let mean = base.behaviour.mean_interval_s;

        let mut baseline = base.clone();
        baseline.behaviour.coupling_strength = 0.0;
        let delta_baseline = final_pair_delta(&simulate(&baseline, 1), mean);

        let mut delay = base.clone();
        delay.behaviour.mode = Mode::EventDelay;
        delay.behaviour.coupling_strength = 0.2;
        let delta_delay = final_pair_delta(&simulate(&delay, 1), mean);
        assert!(
            circ_dist(delta_delay, delta_baseline) > 0.05,
            "event_delay delta {delta_delay} too close to baseline {delta_baseline}"
        );

        let mut advance = base.clone();
        advance.behaviour.mode = Mode::EventAdvance;
        advance.behaviour.coupling_strength = 0.2;
        let delta_advance = final_pair_delta(&simulate(&advance, 1), mean);
        assert!(
            circ_dist(delta_advance, delta_baseline) > 0.05,
            "event_advance delta {delta_advance} too close to baseline {delta_baseline}"
        );

        let mut coupled = base.clone();
        coupled.behaviour.mode = Mode::PhaseCoupled;
        coupled.behaviour.coupling_strength = 0.5;
        let delta_coupled = final_pair_delta(&simulate(&coupled, 1), mean);
        assert!(
            circ_dist(delta_coupled, 0.5) < 0.05,
            "phase_coupled delta {delta_coupled} not near anti-phase"
        );
    }
}
