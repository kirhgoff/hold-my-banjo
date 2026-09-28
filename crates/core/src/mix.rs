use crate::sim::{CallEvent, Scenario};
use std::collections::VecDeque;
use std::f64::consts::{PI, TAU};

pub const MAX_VOICES: usize = 256;
pub const FADE_S: f64 = 0.005;
pub const DISTANCE_REF_M: f64 = 5.0;

pub struct Trigger {
    pub frame: u64,
    pub clip: usize,
    pub step: f64,
    pub gl: f32,
    pub gr: f32,
}

struct Voice {
    clip: usize,
    pos: f64,
    step: f64,
    gl: f32,
    gr: f32,
    active: bool,
}

pub struct Mixer {
    clips: Vec<Vec<f32>>,
    triggers: VecDeque<Trigger>,
    frame: u64,
    voices: Vec<Voice>,
    master_gain: f32,
}

pub fn prepare(mono: Vec<f32>, src_sr: u32, dst_sr: u32) -> Vec<f32> {
    let resampled = if src_sr == dst_sr || mono.is_empty() {
        mono
    } else {
        let dst_len = ((mono.len() as u64 * dst_sr as u64) / src_sr as u64) as usize;
        (0..dst_len)
            .map(|k| {
                let src_pos = k as f64 * src_sr as f64 / dst_sr as f64;
                let i = src_pos as usize;
                if i + 1 >= mono.len() {
                    mono[mono.len() - 1]
                } else {
                    let frac = (src_pos - i as f64) as f32;
                    mono[i] + (mono[i + 1] - mono[i]) * frac
                }
            })
            .collect()
    };

    let mut out = resampled;
    let peak = out.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
    if peak > 0.0 {
        for s in out.iter_mut() {
            *s /= peak;
        }
    }

    let n = out.len();
    let fade_n = ((FADE_S * dst_sr as f64) as usize).min(n);
    for i in 0..fade_n {
        let g = i as f32 / fade_n as f32;
        out[i] *= g;
        out[n - 1 - i] *= g;
    }

    out
}

pub fn synth_bonks(sr: u32) -> Vec<Vec<f32>> {
    [400.0f64, 440.0, 480.0]
        .iter()
        .map(|&f0| {
            let dur = 0.25f64;
            let n = (dur * sr as f64) as usize;
            let mut phase = 0.0f64;
            let mut samples = Vec::with_capacity(n);
            for k in 0..n {
                let t = k as f64 / sr as f64;
                let f = f0 * (1.0 + 0.15 * (-t / 0.02).exp());
                phase += TAU * f / sr as f64;
                let env = (1.0 - (-t / 0.002).exp()) * (-t / 0.06).exp();
                let s = env * (phase.sin() + 0.35 * (2.0 * phase).sin() + 0.1 * (3.0 * phase).sin());
                samples.push(s as f32);
            }
            prepare(samples, sr, sr)
        })
        .collect()
}

pub fn trigger(e: &CallEvent, s: &Scenario, sr: u32) -> Trigger {
    let half_w = s.scene.width_m / 2.0;
    let frame = (e.time_s * sr as f64).round() as u64;
    let d = e.x_m.hypot(e.y_m);
    let att = DISTANCE_REF_M / (DISTANCE_REF_M + d);
    let pan = (s.audio.stereo_width * e.x_m / half_w).clamp(-1.0, 1.0);
    let a = (pan + 1.0) * PI / 4.0;
    let gl = (e.gain * att * a.cos()) as f32;
    let gr = (e.gain * att * a.sin()) as f32;
    Trigger {
        frame,
        clip: e.clip_id,
        step: e.pitch_ratio,
        gl,
        gr,
    }
}

impl Mixer {
    pub fn new(clips: Vec<Vec<f32>>, master_gain: f32) -> Mixer {
        Mixer {
            clips,
            triggers: VecDeque::new(),
            frame: 0,
            voices: (0..MAX_VOICES)
                .map(|_| Voice { clip: 0, pos: 0.0, step: 1.0, gl: 0.0, gr: 0.0, active: false })
                .collect(),
            master_gain,
        }
    }

    pub fn push(&mut self, t: Trigger) {
        self.triggers.push_back(t);
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        for frame in out.chunks_exact_mut(channels) {
            while self.triggers.front().is_some_and(|t| t.frame <= self.frame) {
                let t = self.triggers.pop_front().unwrap();
                if let Some(v) = self.voices.iter_mut().find(|v| !v.active) {
                    v.clip = t.clip;
                    v.pos = 0.0;
                    v.step = t.step;
                    v.gl = t.gl;
                    v.gr = t.gr;
                    v.active = true;
                }
            }

            let mut l = 0.0f32;
            let mut r = 0.0f32;
            for v in self.voices.iter_mut() {
                if !v.active {
                    continue;
                }
                let c = &self.clips[v.clip];
                let i = v.pos as usize;
                if i + 1 >= c.len() {
                    v.active = false;
                    continue;
                }
                let frac = v.pos.fract() as f32;
                let sample = c[i] + (c[i + 1] - c[i]) * frac;
                l += sample * v.gl;
                r += sample * v.gr;
                v.pos += v.step;
            }

            l = (l * self.master_gain).tanh();
            r = (r * self.master_gain).tanh();

            if channels == 1 {
                frame[0] = (l + r) / 2.0;
            } else {
                frame[0] = l;
                frame[1] = r;
                for s in frame.iter_mut().skip(2) {
                    *s = 0.0;
                }
            }

            self.frame += 1;
        }
    }
}

pub fn render_offline(m: &mut Mixer, frames: usize, block: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; frames * 2];
    let mut buf = vec![0.0f32; block * 2];
    let mut pos = 0usize;
    while pos < frames {
        let n = block.min(frames - pos);
        let slice = &mut buf[..n * 2];
        m.render(slice, 2);
        out[pos * 2..(pos + n) * 2].copy_from_slice(slice);
        pos += n;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac06_fifty_simultaneous() {
        let clips = synth_bonks(48000);
        let push_triggers = |m: &mut Mixer| {
            for _ in 0..50 {
                m.push(Trigger { frame: 100, clip: 0, step: 1.0, gl: 1.0, gr: 1.0 });
            }
        };

        let mut m1 = Mixer::new(clips.clone(), 1.0);
        push_triggers(&mut m1);
        let out1 = render_offline(&mut m1, 20000, 64);

        let mut m2 = Mixer::new(clips, 1.0);
        push_triggers(&mut m2);
        let out2 = render_offline(&mut m2, 20000, 4096);

        for &s in &out1 {
            assert!(s.is_finite());
            assert!(s.abs() <= 1.0);
        }
        assert_eq!(out1, out2);
    }

    #[test]
    fn ac07_exact_onset_frame() {
        let clips = vec![vec![0.5f32; 100]];
        let mut m = Mixer::new(clips, 1.0);
        m.push(Trigger { frame: 1000, clip: 0, step: 1.0, gl: 1.0, gr: 1.0 });
        let out = render_offline(&mut m, 1100, 64);

        assert_eq!(out[2 * 999], 0.0);
        assert_ne!(out[2 * 1000], 0.0);
        assert_ne!(out[2 * 1000 + 1], 0.0);
    }

    #[test]
    fn ac02_offline_pcm_identical() {
        let mut s = Scenario::bundled();
        s.duration_s = 5.0;

        let run = || {
            let sr = s.audio.preferred_sample_rate;
            let clips = synth_bonks(sr);
            let events = crate::sim::simulate(&s, clips.len());
            let mut m = Mixer::new(clips, s.audio.master_gain);
            for e in &events {
                m.push(trigger(e, &s, sr));
            }
            render_offline(&mut m, (s.duration_s * sr as f64) as usize + 2 * 48000, 1024)
        };

        assert_eq!(run(), run());
    }
}
