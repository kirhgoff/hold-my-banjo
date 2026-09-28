mod mix;
mod sim;

use clap::{Args, Parser, Subcommand};
use sim::{CallEvent, Mode, Scenario};
use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Duration;

const ABOUT: &str = "Pobblebonk chorus simulator. Scenario `provenance` is `measured` or `experimental`; the bundled default is EXPERIMENTAL (placeholder values, not species-validated). Modes event_delay/event_advance/phase_coupled are HYPOTHETICAL, borrowed from Japanese tree frog models.";

#[derive(Parser)]
#[command(version, about = ABOUT)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Play(RunArgs),
    Render(RunArgs),
    Analyze {
        csv: PathBuf,
        #[arg(long, default_value_t = 0.3)]
        call_s: f64,
    },
}

#[derive(Args)]
struct RunArgs {
    #[arg(long, default_value = "scenarios/default.toml")]
    scenario: PathBuf,
    #[arg(long)]
    frogs: Option<usize>,
    #[arg(long)]
    seed: Option<u64>,
    #[arg(long)]
    duration: Option<f64>,
    #[arg(long, value_enum)]
    mode: Option<Mode>,
    #[arg(long)]
    strength: Option<f64>,
    #[arg(long)]
    mean_interval: Option<f64>,
    #[arg(long)]
    interval_sd: Option<f64>,
    #[arg(long)]
    hearing_radius: Option<f64>,
    #[arg(long)]
    master_gain: Option<f32>,
    #[arg(long)]
    assets: Option<PathBuf>,
    #[arg(long)]
    csv: Option<PathBuf>,
    #[arg(long)]
    wav: Option<PathBuf>,
    #[arg(long)]
    log_events: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Play(args) => {
            let log_events = args.log_events;
            let s = load(&args)?;
            play(s, log_events)
        }
        Cmd::Render(args) => {
            let log_events = args.log_events;
            let s = load(&args)?;
            render(s, log_events)
        }
        Cmd::Analyze { csv, call_s } => analyze(&csv, call_s),
    }
}

fn load(args: &RunArgs) -> Result<Scenario, Box<dyn Error>> {
    let text = std::fs::read_to_string(&args.scenario)?;
    let mut s: Scenario = toml::from_str(&text)?;

    if let Some(v) = args.frogs {
        s.frog_count = v;
    }
    if let Some(v) = args.seed {
        s.seed = v;
    }
    if let Some(v) = args.duration {
        s.duration_s = v;
    }
    if let Some(v) = args.mode {
        s.behaviour.mode = v;
    }
    if let Some(v) = args.strength {
        s.behaviour.coupling_strength = v;
    }
    if let Some(v) = args.mean_interval {
        s.behaviour.mean_interval_s = v;
    }
    if let Some(v) = args.interval_sd {
        s.behaviour.relative_interval_sd = v;
    }
    if let Some(v) = args.hearing_radius {
        s.behaviour.hearing_radius_m = v;
    }
    if let Some(v) = args.master_gain {
        s.audio.master_gain = v;
    }
    if let Some(v) = args.assets.clone() {
        s.audio.asset_dir = v;
    }
    if let Some(v) = args.csv.clone() {
        s.output.event_csv = Some(v);
    }
    if let Some(v) = args.wav.clone() {
        s.output.offline_wav = Some(v);
    }

    s.validate()?;
    Ok(s)
}

fn banner(s: &Scenario, clip_source: &str) {
    println!(
        "provenance={} mode={} frogs={} seed={} clips={}",
        s.provenance.name(),
        s.behaviour.mode.name(),
        s.frog_count,
        s.seed,
        clip_source
    );
    if s.behaviour.mode.hypothetical() {
        println!(
            "HYPOTHETICAL: mode {} is a creative analogy borrowed from Japanese tree frog models, not validated for banjo frogs",
            s.behaviour.mode.name()
        );
    }
    if clip_source == "synthetic_placeholder" {
        println!("SYNTHETIC PLACEHOLDER bonk in use (no WAV assets found in {})", s.audio.asset_dir.display());
    }
}

fn write_csv(s: &Scenario, clip_source: &str, events: &[CallEvent], log_events: bool) -> Result<(), Box<dyn Error>> {
    if let Some(path) = &s.output.event_csv {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, sim::events_csv(s, clip_source, events))?;
    }
    if log_events {
        for e in events {
            println!(
                "{:.6},{},{:.3},{:.3},{},{:.6},{:.6}",
                e.time_s, e.frog_id, e.x_m, e.y_m, e.clip_id, e.pitch_ratio, e.gain
            );
        }
    }
    Ok(())
}

fn play(s: Scenario, log_events: bool) -> Result<(), Box<dyn Error>> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("no default output device")?;
    let supported = device.default_output_config()?;
    if supported.sample_format() != cpal::SampleFormat::F32 {
        return Err(format!("default output device does not support f32 samples (got {:?})", supported.sample_format()).into());
    }
    let config: cpal::StreamConfig = supported.into();
    let sr = config.sample_rate.0;
    let channels = config.channels as usize;

    if sr != s.audio.preferred_sample_rate {
        println!(
            "note: device sample rate {sr} differs from preferred {}",
            s.audio.preferred_sample_rate
        );
    }

    let (clips, clip_source) = mix::load_clips(&s.audio.asset_dir, sr)?;
    banner(&s, clip_source);

    let events = sim::simulate(&s, clips.len());
    write_csv(&s, clip_source, &events, log_events)?;

    let trigs = mix::triggers(&events, &s, sr);
    let max_clip_len = clips.iter().map(|c| c.len()).max().unwrap_or(0);
    let mut mixer = mix::Mixer::new(clips, trigs, s.audio.master_gain);

    let stream = device.build_output_stream(
        &config,
        move |d: &mut [f32], _: &cpal::OutputCallbackInfo| mixer.render(d, channels),
        |e| eprintln!("{e}"),
        None,
    )?;
    stream.play()?;

    let sleep_s = s.duration_s + max_clip_len as f64 / sr as f64 + 0.5;
    std::thread::sleep(Duration::from_secs_f64(sleep_s));

    Ok(())
}

fn render(s: Scenario, log_events: bool) -> Result<(), Box<dyn Error>> {
    let sr = s.audio.preferred_sample_rate;
    let (clips, clip_source) = mix::load_clips(&s.audio.asset_dir, sr)?;
    banner(&s, clip_source);

    let events = sim::simulate(&s, clips.len());
    write_csv(&s, clip_source, &events, log_events)?;

    let trigs = mix::triggers(&events, &s, sr);
    let max_clip_len = clips.iter().map(|c| c.len()).max().unwrap_or(0);
    let frames = (s.duration_s * sr as f64) as usize + 2 * max_clip_len;
    let mut mixer = mix::Mixer::new(clips, trigs, s.audio.master_gain);
    let samples = mix::render_offline(&mut mixer, frames, 1024);

    let wav_path = s.output.offline_wav.clone().ok_or("no output.offline_wav configured and no --wav given")?;
    if let Some(parent) = wav_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: sr,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(&wav_path, spec)?;
    for sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;

    Ok(())
}

fn analyze(csv: &Path, call_s: f64) -> Result<(), Box<dyn Error>> {
    let text = std::fs::read_to_string(csv)?;
    let mut per_frog: BTreeMap<usize, Vec<f64>> = BTreeMap::new();

    for line in text.lines() {
        if line.starts_with('#') {
            println!("{line}");
            continue;
        }
        if line.starts_with("time_s,") || line.trim().is_empty() {
            continue;
        }
        let mut fields = line.split(',');
        let time_s: f64 = fields.next().ok_or("missing time_s")?.parse()?;
        let frog_id: usize = fields.next().ok_or("missing frog_id")?.parse()?;
        per_frog.entry(frog_id).or_default().push(time_s);
    }

    let mut all_times: Vec<f64> = Vec::new();
    for (frog_id, times) in &per_frog {
        all_times.extend(times.iter().copied());
        let n = times.len();
        if n < 2 {
            println!("{frog_id} calls={n} mean_ioi_s=nan sd_ioi_s=nan rate_per_min=nan");
            continue;
        }
        let iois: Vec<f64> = times.windows(2).map(|w| w[1] - w[0]).collect();
        let mean_ioi = iois.iter().sum::<f64>() / iois.len() as f64;
        let var = iois.iter().map(|x| (x - mean_ioi).powi(2)).sum::<f64>() / iois.len() as f64;
        let sd_ioi = var.sqrt();
        let rate = 60.0 * (n as f64 - 1.0) / (times[n - 1] - times[0]);
        println!("{frog_id} calls={n} mean_ioi_s={mean_ioi:.6} sd_ioi_s={sd_ioi:.6} rate_per_min={rate:.6}");
    }

    all_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let m = all_times.len();
    let mut overlapping = 0usize;
    for k in 0..m {
        let before = k > 0 && all_times[k] - all_times[k - 1] < call_s;
        let after = k + 1 < m && all_times[k + 1] - all_times[k] < call_s;
        if before || after {
            overlapping += 1;
        }
    }
    let fraction = if m > 0 { overlapping as f64 / m as f64 } else { 0.0 };
    println!("chorus_overlap_fraction={fraction:.6} (call_s={call_s})");

    Ok(())
}
