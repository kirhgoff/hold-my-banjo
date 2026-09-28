pub mod mix;
pub mod sim;

use mix::{synth_bonks, trigger, Mixer};
use sim::{CallEvent, Scenario, Sim};

pub struct Chorus {
    sim: Sim,
    mixer: Mixer,
    sr: u32,
    events: Vec<CallEvent>,
}

impl Chorus {
    pub fn new(s: Scenario, sr: u32) -> Chorus {
        let clips = synth_bonks(sr);
        let master_gain = s.audio.master_gain;
        let sim = Sim::new(s, clips.len());
        Chorus { sim, mixer: Mixer::new(clips, master_gain), sr, events: Vec::new() }
    }

    pub fn sim(&self) -> &Sim {
        &self.sim
    }

    pub fn sim_mut(&mut self) -> &mut Sim {
        &mut self.sim
    }

    pub fn render(&mut self, out: &mut [f32]) -> &[CallEvent] {
        let frames = (out.len() / 2) as u64;
        let target_s = (self.mixer.frame() + frames) as f64 / self.sr as f64;
        self.events.clear();
        while self.sim.time_s() <= target_s {
            self.sim.step(&mut self.events);
        }
        for e in &self.events {
            self.mixer.push(trigger(e, self.sim.scenario(), self.sr));
        }
        self.mixer.render(out, 2);
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_matches_batch() {
        let mut s = Scenario::bundled();
        s.duration_s = 5.0;
        let sr = s.audio.preferred_sample_rate;
        let frames = (s.duration_s * sr as f64) as usize;
        let clips = synth_bonks(sr);
        let events = sim::simulate(&s, clips.len());
        let mut m = Mixer::new(clips, s.audio.master_gain);
        for e in &events {
            m.push(trigger(e, &s, sr));
        }
        let batch = mix::render_offline(&mut m, frames, 1024);
        let mut chorus = Chorus::new(s, sr);
        let mut streamed = Vec::new();
        let mut block = vec![0.0f32; 256];
        while streamed.len() < frames * 2 {
            chorus.render(&mut block);
            streamed.extend_from_slice(&block);
        }
        assert_eq!(&streamed[..frames * 2], &batch[..frames * 2]);
    }
}
